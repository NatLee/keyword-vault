//! 關鍵字的新增、查詢、修改和刪除，以及組合關鍵字的連動更新。

use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{QueryBuilder, Sqlite, SqliteConnection, SqlitePool};

use crate::{
    auth::CurrentUser,
    combo::Logic,
    db,
    error::{AppError, AppResult},
    state::AppState,
    syntax::{self, MAX_EXPANDED_CHARS, MAX_EXPRESSION_CHARS},
};

const DEFAULT_PAGE_SIZE: i64 = 20;
const MAX_PAGE_SIZE: i64 = 100;
const MAX_TAGS: usize = 10;

#[derive(sqlx::FromRow)]
pub(super) struct KeywordRow {
    pub id: i64,
    pub theme: String,
    pub expression: String,
    pub logic: Option<String>,
    pub visibility: String,
    pub owner_id: i64,
    pub owner_username: String,
    pub owner_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub favorite: i64,
    pub comment_count: i64,
}

#[derive(Serialize)]
pub struct Owner {
    pub id: i64,
    pub username: String,
    pub display_name: String,
}

#[derive(Serialize)]
pub struct Keyword {
    pub id: i64,
    pub theme: String,
    pub expression: String,
    /// 組合關鍵字的組合方式；一般關鍵字是 `null`。
    pub logic: Option<String>,
    pub is_combination: bool,
    pub visibility: String,
    pub owner: Owner,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub comment_count: i64,
    pub created_at: String,
    pub updated_at: String,
    pub can_edit: bool,
    pub can_delete: bool,
}

/// 「這個人看得到這個關鍵字」的條件：自己的、公開的，或是同群組的人分享到群組的。
pub(super) fn push_visible(qb: &mut QueryBuilder<'_, Sqlite>, user_id: i64) {
    qb.push("(k.owner_id = ")
        .push_bind(user_id)
        .push(
            " OR k.visibility = 'public' OR (k.visibility = 'group' AND EXISTS (
                SELECT 1 FROM group_members a JOIN group_members b ON a.group_id = b.group_id
                WHERE a.user_id = k.owner_id AND b.user_id = ",
        )
        .push_bind(user_id)
        .push(")))");
}

pub(super) fn select(user_id: i64) -> QueryBuilder<'static, Sqlite> {
    let mut qb = QueryBuilder::new(
        "SELECT k.id, k.theme, k.expression, k.logic, k.visibility, k.owner_id,
                u.username AS owner_username, u.display_name AS owner_name,
                k.created_at, k.updated_at,
                EXISTS (SELECT 1 FROM favorites f WHERE f.keyword_id = k.id AND f.user_id = ",
    );
    qb.push_bind(user_id).push(
        ") AS favorite,
                (SELECT COUNT(*) FROM comments c WHERE c.keyword_id = k.id) AS comment_count
         FROM keywords k JOIN users u ON u.id = k.owner_id WHERE ",
    );
    qb
}

/// 補上標籤，轉成 API 的格式。
pub(super) async fn hydrate(
    db: &SqlitePool,
    user: &CurrentUser,
    rows: Vec<KeywordRow>,
) -> AppResult<Vec<Keyword>> {
    let mut tags: HashMap<i64, Vec<String>> = HashMap::new();
    if !rows.is_empty() {
        let mut qb = QueryBuilder::new(
            "SELECT kt.keyword_id, t.name FROM keyword_tags kt JOIN tags t ON t.id = kt.tag_id
             WHERE kt.keyword_id IN (",
        );
        let mut ids = qb.separated(", ");
        for row in &rows {
            ids.push_bind(row.id);
        }
        qb.push(") ORDER BY t.name");
        let pairs: Vec<(i64, String)> = qb.build_query_as().fetch_all(db).await?;
        for (id, name) in pairs {
            tags.entry(id).or_default().push(name);
        }
    }
    Ok(rows
        .into_iter()
        .map(|r| Keyword {
            tags: tags.remove(&r.id).unwrap_or_default(),
            is_combination: r.logic.is_some(),
            can_edit: r.owner_id == user.id,
            can_delete: r.owner_id == user.id || user.is_admin(),
            favorite: r.favorite != 0,
            owner: Owner {
                id: r.owner_id,
                username: r.owner_username,
                display_name: r.owner_name,
            },
            id: r.id,
            theme: r.theme,
            expression: r.expression,
            logic: r.logic,
            visibility: r.visibility,
            comment_count: r.comment_count,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect())
}

/// 讀一個看得到的關鍵字；看不到和不存在都回 404。
pub(super) async fn load_one(db: &SqlitePool, user: &CurrentUser, id: i64) -> AppResult<Keyword> {
    let mut qb = select(user.id);
    push_visible(&mut qb, user.id);
    qb.push(" AND k.id = ").push_bind(id);
    let rows = qb.build_query_as::<KeywordRow>().fetch_all(db).await?;
    hydrate(db, user, rows)
        .await?
        .pop()
        .ok_or_else(|| AppError::not_found("找不到這個關鍵字"))
}

#[derive(Deserialize, Default)]
pub struct ListParams {
    /// 在主題和語法裡找
    pub q: Option<String>,
    pub tag: Option<String>,
    /// `all`（預設）、`mine`、`shared`（別人分享的）、`favorites`
    pub scope: Option<String>,
    /// `plain` 或 `combination`
    pub kind: Option<String>,
    /// `updated`（預設）、`created`、`theme`
    pub sort: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub(super) fn push_filters(
    qb: &mut QueryBuilder<'_, Sqlite>,
    user: &CurrentUser,
    p: &ListParams,
) -> AppResult<()> {
    push_visible(qb, user.id);
    match p.scope.as_deref().unwrap_or("all") {
        "all" => {}
        "mine" => {
            qb.push(" AND k.owner_id = ").push_bind(user.id);
        }
        "shared" => {
            qb.push(" AND k.owner_id != ").push_bind(user.id);
        }
        "favorites" => {
            qb.push(" AND EXISTS (SELECT 1 FROM favorites fv WHERE fv.keyword_id = k.id AND fv.user_id = ")
                .push_bind(user.id)
                .push(")");
        }
        other => return Err(AppError::bad(format!("不認得的 scope：{other}"))),
    }
    match p.kind.as_deref() {
        None | Some("") | Some("all") => {}
        Some("plain") => {
            qb.push(" AND k.logic IS NULL");
        }
        Some("combination") => {
            qb.push(" AND k.logic IS NOT NULL");
        }
        Some(other) => return Err(AppError::bad(format!("不認得的 kind：{other}"))),
    }
    if let Some(q) = p.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let like = format!(
            "%{}%",
            q.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        qb.push(" AND (k.theme LIKE ")
            .push_bind(like.clone())
            .push(" ESCAPE '\\' OR k.expression LIKE ")
            .push_bind(like)
            .push(" ESCAPE '\\')");
    }
    if let Some(tag) = p.tag.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        qb.push(
            " AND EXISTS (SELECT 1 FROM keyword_tags kt JOIN tags t ON t.id = kt.tag_id
                          WHERE kt.keyword_id = k.id AND t.name = ",
        )
        .push_bind(tag.to_owned())
        .push(")");
    }
    Ok(())
}

pub(super) fn push_order(qb: &mut QueryBuilder<'_, Sqlite>, sort: Option<&str>) -> AppResult<()> {
    qb.push(match sort.unwrap_or("updated") {
        "updated" => " ORDER BY k.updated_at DESC, k.id DESC",
        "created" => " ORDER BY k.created_at DESC, k.id DESC",
        "theme" => " ORDER BY k.theme COLLATE NOCASE, k.id",
        other => return Err(AppError::bad(format!("不認得的 sort：{other}"))),
    });
    Ok(())
}

pub async fn list(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(p): Query<ListParams>,
) -> AppResult<Json<Value>> {
    let page_size = p
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let page = p.page.unwrap_or(1).max(1);
    let offset = (page - 1)
        .checked_mul(page_size)
        .ok_or_else(|| AppError::bad("頁碼太大"))?;

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM keywords k WHERE ");
    push_filters(&mut count, &user, &p)?;
    let total: i64 = count.build_query_scalar().fetch_one(&state.db).await?;

    let mut qb = select(user.id);
    push_filters(&mut qb, &user, &p)?;
    push_order(&mut qb, p.sort.as_deref())?;
    qb.push(" LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows = qb
        .build_query_as::<KeywordRow>()
        .fetch_all(&state.db)
        .await?;
    Ok(Json(json!({
        "items": hydrate(&state.db, &user, rows).await?,
        "total": total,
        "page": page,
        "page_size": page_size,
    })))
}

/// 隨機挑一個有分享出來的關鍵字；沒有的話回 `null`。
pub async fn random(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<Option<Keyword>>> {
    let mut qb = select(user.id);
    push_visible(&mut qb, user.id);
    qb.push(" AND k.visibility != 'private' ORDER BY RANDOM() LIMIT 1");
    let rows = qb
        .build_query_as::<KeywordRow>()
        .fetch_all(&state.db)
        .await?;
    Ok(Json(hydrate(&state.db, &user, rows).await?.pop()))
}

pub struct NewKeyword {
    pub theme: String,
    pub expression: Option<String>,
    pub logic: Option<String>,
    pub visibility: String,
    pub tags: Vec<String>,
}

fn clean_theme(theme: &str) -> AppResult<String> {
    let theme = theme.trim();
    if theme.is_empty() {
        return Err(AppError::bad("請填主題"));
    }
    if theme.chars().count() > 100 {
        return Err(AppError::bad("主題最多 100 個字元"));
    }
    Ok(theme.to_owned())
}

fn check_visibility(visibility: &str) -> AppResult<()> {
    if matches!(visibility, "private" | "group" | "public") {
        Ok(())
    } else {
        Err(AppError::bad("分享範圍只能是 private、group 或 public"))
    }
}

/// 去掉空白、開頭的 `#` 和重複的標籤。
pub fn clean_tags(tags: &[String]) -> AppResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#').trim();
        if tag.is_empty() || out.iter().any(|t| t == tag) {
            continue;
        }
        if tag.chars().count() > 30 {
            return Err(AppError::bad("標籤最多 30 個字元"));
        }
        out.push(tag.to_owned());
    }
    if out.len() > MAX_TAGS {
        return Err(AppError::bad(format!("一個關鍵字最多 {MAX_TAGS} 個標籤")));
    }
    Ok(out)
}

/// 檢查一般關鍵字的語法，回傳去掉前後空白的內容。
fn check_expression(expression: &str, slop: u32) -> AppResult<String> {
    let expression = expression.trim();
    if expression.chars().count() > MAX_EXPRESSION_CHARS {
        return Err(AppError::bad(format!(
            "關鍵字最多 {MAX_EXPRESSION_CHARS} 個字元"
        )));
    }
    syntax::compile(expression, slop, true)
        .map_err(|e| AppError::bad(format!("關鍵字語法錯誤：{}", syntax::error_text(&e))))?;
    Ok(expression.to_owned())
}

/// 解析組合方式並展開。引用的關鍵字都要是這個人看得到的。
async fn resolve_logic(
    conn: &mut SqliteConnection,
    user_id: i64,
    logic: &str,
) -> AppResult<(Logic, String)> {
    let logic = Logic::parse(logic).map_err(|e| AppError::bad(format!("組合方式有誤：{e}")))?;
    let ids = logic.ids();
    let mut qb = QueryBuilder::new("SELECT k.id, k.expression FROM keywords k WHERE k.id IN (");
    let mut list = qb.separated(", ");
    for id in &ids {
        list.push_bind(*id);
    }
    qb.push(") AND ");
    push_visible(&mut qb, user_id);
    let rows: Vec<(i64, String)> = qb.build_query_as().fetch_all(&mut *conn).await?;
    let expressions: HashMap<i64, String> = rows.into_iter().collect();
    if let Some(missing) = ids.iter().find(|id| !expressions.contains_key(id)) {
        return Err(AppError::bad(format!("找不到關鍵字 #{missing}")));
    }
    let expanded = logic.expand(&expressions);
    if expanded.chars().count() > MAX_EXPANDED_CHARS {
        return Err(AppError::bad("組合之後的關鍵字太長了"));
    }
    es_keyword_query::validate(&expanded)
        .map_err(|err| AppError::bad(format!("組合之後的語法有誤：{}", err.message_zh())))?;
    Ok((logic, expanded))
}

async fn set_refs(conn: &mut SqliteConnection, id: i64, refs: &[i64]) -> AppResult<()> {
    sqlx::query("DELETE FROM keyword_refs WHERE keyword_id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    for ref_id in refs {
        sqlx::query("INSERT INTO keyword_refs (keyword_id, ref_id) VALUES (?, ?)")
            .bind(id)
            .bind(ref_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

async fn set_tags(conn: &mut SqliteConnection, id: i64, tags: &[String]) -> AppResult<()> {
    sqlx::query("DELETE FROM keyword_tags WHERE keyword_id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    for tag in tags {
        sqlx::query("INSERT OR IGNORE INTO tags (name) VALUES (?)")
            .bind(tag)
            .execute(&mut *conn)
            .await?;
        sqlx::query(
            "INSERT INTO keyword_tags (keyword_id, tag_id) SELECT ?, id FROM tags WHERE name = ?",
        )
        .bind(id)
        .bind(tag)
        .execute(&mut *conn)
        .await?;
    }
    sqlx::query("DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM keyword_tags)")
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// 新增一個關鍵字（一般或組合），回傳編號。匯入和範例資料也用這個。
pub async fn insert(
    conn: &mut SqliteConnection,
    user_id: i64,
    slop: u32,
    new: NewKeyword,
    at: &str,
) -> AppResult<i64> {
    let theme = clean_theme(&new.theme)?;
    check_visibility(&new.visibility)?;
    let tags = clean_tags(&new.tags)?;
    let (expression, logic) = match (&new.expression, &new.logic) {
        (Some(expression), None) => (check_expression(expression, slop)?, None),
        (None, Some(logic)) => {
            let (logic, expanded) = resolve_logic(conn, user_id, logic).await?;
            (expanded, Some(logic))
        }
        _ => {
            return Err(AppError::bad(
                "請給關鍵字語法（expression）或組合方式（logic）其中一個",
            ))
        }
    };
    let id = sqlx::query(
        "INSERT INTO keywords (theme, expression, logic, owner_id, visibility, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(theme)
    .bind(expression)
    .bind(logic.as_ref().map(Logic::canonical))
    .bind(user_id)
    .bind(&new.visibility)
    .bind(at)
    .bind(at)
    .execute(&mut *conn)
    .await?
    .last_insert_rowid();
    if let Some(logic) = &logic {
        set_refs(conn, id, &logic.ids()).await?;
    }
    set_tags(conn, id, &tags).await?;
    Ok(id)
}

#[derive(Deserialize)]
pub struct CreateBody {
    theme: String,
    expression: Option<String>,
    logic: Option<String>,
    visibility: Option<String>,
    tags: Option<Vec<String>>,
}

pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(body): Json<CreateBody>,
) -> AppResult<Json<Keyword>> {
    let new = NewKeyword {
        theme: body.theme,
        expression: body.expression,
        logic: body.logic,
        visibility: body.visibility.unwrap_or_else(|| "private".into()),
        tags: body.tags.unwrap_or_default(),
    };
    let mut tx = state.db.begin().await?;
    let id = insert(&mut tx, user.id, state.config.default_slop, new, &db::now()).await?;
    tx.commit().await?;
    Ok(Json(load_one(&state.db, &user, id).await?))
}

/// 直接或間接用到 `id` 的所有組合關鍵字。
async fn descendants(conn: &mut SqliteConnection, id: i64) -> AppResult<Vec<i64>> {
    let mut found: Vec<i64> = Vec::new();
    let mut queue = vec![id];
    while let Some(current) = queue.pop() {
        let users: Vec<i64> =
            sqlx::query_scalar("SELECT keyword_id FROM keyword_refs WHERE ref_id = ?")
                .bind(current)
                .fetch_all(&mut *conn)
                .await?;
        for user in users {
            if !found.contains(&user) {
                found.push(user);
                queue.push(user);
            }
        }
    }
    Ok(found)
}

async fn record_history(
    conn: &mut SqliteConnection,
    id: i64,
    old_expression: &str,
    user_id: i64,
    at: &str,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO history (keyword_id, expression, user_id, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(id)
    .bind(old_expression)
    .bind(user_id)
    .bind(at)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// `id` 的語法變了之後，重新展開所有用到它的組合關鍵字。
/// 照相依順序處理：一個組合關鍵字引用的東西都更新完了，才輪到它。
/// 回傳實際有變動的數量。
async fn cascade(conn: &mut SqliteConnection, id: i64, editor: i64, at: &str) -> AppResult<usize> {
    let order = descendants(conn, id).await?;
    let mut pending: HashSet<i64> = order.iter().copied().collect();
    let mut changed = 0;
    while !pending.is_empty() {
        let mut progressed = false;
        for target in &order {
            if !pending.contains(target) {
                continue;
            }
            let refs: Vec<(i64, String)> = sqlx::query_as(
                "SELECT k.id, k.expression FROM keyword_refs r JOIN keywords k ON k.id = r.ref_id
                 WHERE r.keyword_id = ?",
            )
            .bind(target)
            .fetch_all(&mut *conn)
            .await?;
            if refs.iter().any(|(ref_id, _)| pending.contains(ref_id)) {
                continue;
            }
            pending.remove(target);
            progressed = true;
            let (theme, logic, old): (String, Option<String>, String) =
                sqlx::query_as("SELECT theme, logic, expression FROM keywords WHERE id = ?")
                    .bind(target)
                    .fetch_one(&mut *conn)
                    .await?;
            let Some(Ok(logic)) = logic.as_deref().map(Logic::parse) else {
                continue;
            };
            let expanded = logic.expand(&refs.into_iter().collect());
            if expanded == old {
                continue;
            }
            if expanded.chars().count() > MAX_EXPANDED_CHARS {
                return Err(AppError::bad(format!(
                    "這樣改的話，組合關鍵字「{theme}」會太長"
                )));
            }
            es_keyword_query::validate(&expanded).map_err(|err| {
                AppError::bad(format!(
                    "組合關鍵字「{theme}」無法更新：{}",
                    err.message_zh()
                ))
            })?;
            record_history(conn, *target, &old, editor, at).await?;
            sqlx::query("UPDATE keywords SET expression = ?, updated_at = ? WHERE id = ?")
                .bind(expanded)
                .bind(at)
                .bind(target)
                .execute(&mut *conn)
                .await?;
            changed += 1;
        }
        if !progressed {
            // 相依關係不會有循環（新增和修改時都檢查過），這裡只是保險
            break;
        }
    }
    Ok(changed)
}

#[derive(Deserialize)]
pub struct UpdateBody {
    theme: Option<String>,
    expression: Option<String>,
    logic: Option<String>,
    visibility: Option<String>,
    tags: Option<Vec<String>>,
}

pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateBody>,
) -> AppResult<Json<Value>> {
    let current = load_one(&state.db, &user, id).await?;
    if !current.can_edit {
        return Err(AppError::forbidden("只有擁有者可以修改這個關鍵字"));
    }
    let now = db::now();
    let mut tx = state.db.begin().await?;

    if let Some(theme) = &body.theme {
        sqlx::query("UPDATE keywords SET theme = ? WHERE id = ?")
            .bind(clean_theme(theme)?)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(visibility) = &body.visibility {
        check_visibility(visibility)?;
        sqlx::query("UPDATE keywords SET visibility = ? WHERE id = ?")
            .bind(visibility)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(tags) = &body.tags {
        set_tags(&mut tx, id, &clean_tags(tags)?).await?;
    }

    let expression = if current.is_combination {
        if body.expression.is_some() {
            return Err(AppError::bad("組合關鍵字的語法是展開出來的，請改組合方式"));
        }
        match &body.logic {
            Some(logic) => {
                let (logic, expanded) = resolve_logic(&mut tx, user.id, logic).await?;
                let ids = logic.ids();
                let below = descendants(&mut tx, id).await?;
                if ids.iter().any(|r| *r == id || below.contains(r)) {
                    return Err(AppError::bad(
                        "組合關鍵字不能用到自己，或用到引用了它的關鍵字",
                    ));
                }
                sqlx::query("UPDATE keywords SET logic = ? WHERE id = ?")
                    .bind(logic.canonical())
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                set_refs(&mut tx, id, &ids).await?;
                Some(expanded)
            }
            None => None,
        }
    } else {
        if body.logic.is_some() {
            return Err(AppError::bad("一般關鍵字不能改成組合關鍵字"));
        }
        match &body.expression {
            Some(e) => Some(check_expression(e, state.config.default_slop)?),
            None => None,
        }
    };

    let mut cascaded = 0;
    if let Some(expression) = expression.filter(|e| *e != current.expression) {
        record_history(&mut tx, id, &current.expression, user.id, &now).await?;
        sqlx::query("UPDATE keywords SET expression = ? WHERE id = ?")
            .bind(expression)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        cascaded = cascade(&mut tx, id, user.id, &now).await?;
    }
    sqlx::query("UPDATE keywords SET updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(json!({
        "keyword": load_one(&state.db, &user, id).await?,
        // 跟著更新的組合關鍵字數量
        "cascaded": cascaded,
    })))
}

#[derive(Deserialize)]
pub struct RemoveParams {
    /// 連同用到它的組合關鍵字一起刪
    cascade: Option<bool>,
}

pub async fn remove(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i64>,
    Query(params): Query<RemoveParams>,
) -> AppResult<Json<Value>> {
    let current = load_one(&state.db, &user, id).await?;
    if !current.can_delete {
        return Err(AppError::forbidden("只有擁有者或管理員可以刪除這個關鍵字"));
    }
    let mut tx = state.db.begin().await?;
    let below = descendants(&mut tx, id).await?;
    if !below.is_empty() {
        if !params.cascade.unwrap_or(false) {
            return Err(AppError::conflict(format!(
                "有 {} 個組合關鍵字用到它；要一起刪除的話請加上 cascade=true",
                below.len()
            )));
        }
        if !user.is_admin() {
            let mut qb = QueryBuilder::new("SELECT COUNT(*) FROM keywords WHERE owner_id != ");
            qb.push_bind(user.id).push(" AND id IN (");
            let mut list = qb.separated(", ");
            for d in &below {
                list.push_bind(*d);
            }
            qb.push(")");
            let others: i64 = qb.build_query_scalar().fetch_one(&mut *tx).await?;
            if others > 0 {
                return Err(AppError::conflict(format!(
                    "有 {others} 個別人的組合關鍵字用到它，不能刪除"
                )));
            }
        }
    }
    // 先拆掉引用關係，再刪關鍵字本身
    let mut all = below.clone();
    all.push(id);
    for target in &all {
        sqlx::query("DELETE FROM keyword_refs WHERE keyword_id = ?")
            .bind(target)
            .execute(&mut *tx)
            .await?;
    }
    for target in &all {
        sqlx::query("DELETE FROM keywords WHERE id = ?")
            .bind(target)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM keyword_tags)")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "deleted": all.len() })))
}

#[derive(Serialize)]
struct Linked {
    id: i64,
    /// 看不到的關鍵字不給主題
    theme: Option<String>,
}

async fn linked(
    db: &SqlitePool,
    user_id: i64,
    id: i64,
    dependents: bool,
) -> AppResult<Vec<Linked>> {
    let mut qb = QueryBuilder::new("SELECT k.id, k.theme, ");
    push_visible(&mut qb, user_id);
    qb.push(if dependents {
        " AS visible FROM keyword_refs r JOIN keywords k ON k.id = r.keyword_id WHERE r.ref_id = "
    } else {
        " AS visible FROM keyword_refs r JOIN keywords k ON k.id = r.ref_id WHERE r.keyword_id = "
    })
    .push_bind(id)
    .push(" ORDER BY k.id");
    let rows: Vec<(i64, String, i64)> = qb.build_query_as().fetch_all(db).await?;
    Ok(rows
        .into_iter()
        .map(|(id, theme, visible)| Linked {
            id,
            theme: (visible != 0).then_some(theme),
        })
        .collect())
}

#[derive(Serialize)]
pub struct Detail {
    #[serde(flatten)]
    keyword: Keyword,
    query_string: String,
    terms: Vec<String>,
    clauses: Vec<syntax::Clause>,
    /// 這個組合關鍵字用到的關鍵字
    refs: Vec<Linked>,
    /// 直接用到這個關鍵字的組合關鍵字
    dependents: Vec<Linked>,
}

pub async fn detail(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Detail>> {
    let keyword = load_one(&state.db, &user, id).await?;
    let compiled = syntax::compile(&keyword.expression, state.config.default_slop, true)
        .map_err(|e| anyhow::anyhow!("資料庫裡的關鍵字 {id} 語法有誤：{e}"))?;
    Ok(Json(Detail {
        query_string: compiled.query_string,
        terms: compiled.terms,
        clauses: compiled.clauses,
        refs: linked(&state.db, user.id, id, false).await?,
        dependents: linked(&state.db, user.id, id, true).await?,
        keyword,
    }))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct HistoryEntry {
    id: i64,
    /// 修改之前的語法
    expression: String,
    user_name: Option<String>,
    created_at: String,
}

pub async fn history(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<HistoryEntry>>> {
    load_one(&state.db, &user, id).await?;
    let entries = sqlx::query_as::<_, HistoryEntry>(
        "SELECT h.id, h.expression, u.display_name AS user_name, h.created_at
         FROM history h LEFT JOIN users u ON u.id = h.user_id
         WHERE h.keyword_id = ? ORDER BY h.id DESC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(entries))
}

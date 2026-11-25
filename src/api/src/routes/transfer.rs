//! CSV 匯入和匯出。

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use super::keywords::{
    self, hydrate, push_filters, push_order, select, KeywordRow, ListParams, NewKeyword,
};
use crate::{
    auth::CurrentUser,
    db,
    error::{AppError, AppResult},
    state::AppState,
    syntax,
};

const MAX_IMPORT_ROWS: usize = 2_000;
const MAX_EXPORT_ROWS: i64 = 10_000;

fn column(name: &str) -> Option<&'static str> {
    match name
        .trim()
        .trim_start_matches('\u{feff}')
        .to_lowercase()
        .as_str()
    {
        "theme" | "主題" => Some("theme"),
        "expression" | "keywords" | "關鍵字" => Some("expression"),
        "visibility" | "分享範圍" | "是否分享（是/否）" | "是否分享" => {
            Some("visibility")
        }
        "tags" | "標籤" => Some("tags"),
        _ => None,
    }
}

fn visibility(text: &str) -> Result<&'static str, String> {
    match text.trim().to_lowercase().as_str() {
        "" | "private" | "私人" | "否" => Ok("private"),
        "group" | "群組" => Ok("group"),
        "public" | "公開" | "是" => Ok("public"),
        other => Err(format!(
            "看不懂分享範圍「{other}」，請用 私人、群組 或 公開"
        )),
    }
}

/// 匯入 CSV（請求的內容就是 CSV 文字）。只要有一列不對，整份都不匯入。
///
/// 欄位：`主題`、`關鍵字`，以及選填的 `分享範圍`（私人／群組／公開）和 `標籤`（用 `;` 分隔）。
pub async fn import(
    State(state): State<AppState>,
    user: CurrentUser,
    body: String,
) -> AppResult<Response> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(body.as_bytes());
    let headers: Vec<Option<&str>> = reader
        .headers()
        .map_err(|e| AppError::bad(format!("讀不懂這個 CSV：{e}")))?
        .iter()
        .map(column)
        .collect();
    let index = |name: &str| headers.iter().position(|h| *h == Some(name));
    let (Some(theme_at), Some(expression_at)) = (index("theme"), index("expression")) else {
        return Err(AppError::bad(
            "CSV 的第一列要有「主題」和「關鍵字」兩個欄位",
        ));
    };
    let (visibility_at, tags_at) = (index("visibility"), index("tags"));

    let now = db::now();
    let mut tx = state.db.begin().await?;
    let mut created = 0;
    let mut problems = Vec::new();
    for (i, record) in reader.records().enumerate() {
        // 第一列是欄位名稱，所以資料從第 2 列開始
        let line = i + 2;
        if i >= MAX_IMPORT_ROWS {
            return Err(AppError::bad(format!("一次最多匯入 {MAX_IMPORT_ROWS} 列")));
        }
        let record = record.map_err(|e| AppError::bad(format!("第 {line} 列讀不懂：{e}")))?;
        let field = |at: Option<usize>| at.and_then(|at| record.get(at)).unwrap_or_default();
        let theme = field(Some(theme_at));
        if record.iter().all(str::is_empty) {
            continue;
        }
        let result = match visibility(field(visibility_at)) {
            Ok(visibility) => {
                let new = NewKeyword {
                    theme: theme.to_owned(),
                    expression: Some(field(Some(expression_at)).to_owned()),
                    logic: None,
                    visibility: visibility.to_owned(),
                    tags: field(tags_at)
                        .split([';', '；'])
                        .map(str::to_owned)
                        .collect(),
                };
                keywords::insert(&mut tx, user.id, state.config.default_slop, new, &now)
                    .await
                    .map(|_| ())
            }
            Err(message) => Err(AppError::bad(message)),
        };
        match result {
            Ok(()) => created += 1,
            Err(AppError::BadRequest(message)) => {
                problems.push(json!({ "line": line, "theme": theme, "message": message }));
            }
            Err(other) => return Err(other),
        }
    }
    if !problems.is_empty() {
        // tx 在這裡被丟掉，等於 rollback
        let body = json!({
            "error": format!("有 {} 列有問題，這次沒有匯入任何關鍵字", problems.len()),
            "rows": problems,
        });
        return Ok((StatusCode::BAD_REQUEST, Json(body)).into_response());
    }
    tx.commit().await?;
    Ok(Json(json!({ "created": created })).into_response())
}

/// 匯出成 CSV。篩選條件和列表一樣；多一欄編譯好的 query_string。
pub async fn export(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(params): Query<ListParams>,
) -> AppResult<Response> {
    let mut qb = select(user.id);
    push_filters(&mut qb, &user, &params)?;
    push_order(&mut qb, params.sort.as_deref())?;
    qb.push(" LIMIT ").push_bind(MAX_EXPORT_ROWS);
    let rows = qb
        .build_query_as::<KeywordRow>()
        .fetch_all(&state.db)
        .await?;
    let items = hydrate(&state.db, &user, rows).await?;

    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "主題",
        "關鍵字",
        "分享範圍",
        "標籤",
        "擁有者",
        "query_string",
    ])?;
    for item in items {
        let query_string = syntax::compile(&item.expression, state.config.default_slop, true)
            .map(|c| c.query_string)
            .unwrap_or_default();
        let visibility = match item.visibility.as_str() {
            "public" => "公開",
            "group" => "群組",
            _ => "私人",
        };
        writer.write_record([
            item.theme.as_str(),
            item.expression.as_str(),
            visibility,
            item.tags.join(";").as_str(),
            item.owner.display_name.as_str(),
            query_string.as_str(),
        ])?;
    }
    // 開頭加 BOM，Excel 才會用 UTF-8 開
    let mut body = b"\xEF\xBB\xBF".to_vec();
    body.extend(writer.into_inner().map_err(|e| anyhow::anyhow!("{e}"))?);
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"keywords.csv\"",
            ),
        ],
        body,
    )
        .into_response())
}

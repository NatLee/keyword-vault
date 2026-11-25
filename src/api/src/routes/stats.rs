//! 總覽頁的數字和標籤列表。

use std::collections::HashMap;

use axum::{extract::State, Json};
use chrono::{Datelike, Months, Utc};
use serde_json::{json, Value};
use sqlx::QueryBuilder;

use super::keywords::{hydrate, push_visible, select, KeywordRow};
use crate::{auth::CurrentUser, error::AppResult, state::AppState};

/// 看得到的關鍵字裡，每個標籤用了幾次。
pub async fn tags(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Value>> {
    let mut qb = QueryBuilder::new(
        "SELECT t.name, COUNT(*) FROM keyword_tags kt
         JOIN tags t ON t.id = kt.tag_id JOIN keywords k ON k.id = kt.keyword_id WHERE ",
    );
    push_visible(&mut qb, user.id);
    qb.push(" GROUP BY t.id ORDER BY COUNT(*) DESC, t.name");
    let rows: Vec<(String, i64)> = qb.build_query_as().fetch_all(&state.db).await?;
    Ok(Json(
        rows.into_iter()
            .map(|(name, count)| json!({ "name": name, "count": count }))
            .collect(),
    ))
}

pub async fn overview(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Value>> {
    let db = &state.db;

    let mut qb = QueryBuilder::new("SELECT COUNT(*), COALESCE(SUM(k.owner_id = ");
    qb.push_bind(user.id)
        .push("), 0), COALESCE(SUM(k.owner_id = ")
        .push_bind(user.id)
        .push(" AND k.visibility = 'private'), 0), COALESCE(SUM(k.logic IS NOT NULL), 0) FROM keywords k WHERE ");
    push_visible(&mut qb, user.id);
    let (total, mine, mine_private, combinations): (i64, i64, i64, i64) =
        qb.build_query_as().fetch_one(db).await?;

    let mut qb = QueryBuilder::new(
        "SELECT COUNT(*) FROM favorites f JOIN keywords k ON k.id = f.keyword_id WHERE f.user_id = ",
    );
    qb.push_bind(user.id).push(" AND ");
    push_visible(&mut qb, user.id);
    let favorites: i64 = qb.build_query_scalar().fetch_one(db).await?;

    // 最近 12 個月每個月新增幾個（含這個月）
    let this_month = Utc::now().date_naive().with_day(1).unwrap_or_default();
    let months: Vec<String> = (0..12u32)
        .rev()
        .filter_map(|back| this_month.checked_sub_months(Months::new(back)))
        .map(|d| format!("{:04}-{:02}", d.year(), d.month()))
        .collect();
    let mut qb =
        QueryBuilder::new("SELECT substr(k.created_at, 1, 7), COUNT(*) FROM keywords k WHERE ");
    push_visible(&mut qb, user.id);
    qb.push(" AND k.created_at >= ")
        .push_bind(months.first().cloned().unwrap_or_default())
        .push(" GROUP BY 1");
    let counts: HashMap<String, i64> = qb
        .build_query_as::<(String, i64)>()
        .fetch_all(db)
        .await?
        .into_iter()
        .collect();
    let monthly: Vec<Value> = months
        .iter()
        .map(|m| json!({ "month": m, "count": counts.get(m).copied().unwrap_or(0) }))
        .collect();

    let mut qb = QueryBuilder::new(
        "SELECT u.display_name, COUNT(*) FROM keywords k JOIN users u ON u.id = k.owner_id WHERE ",
    );
    push_visible(&mut qb, user.id);
    qb.push(" GROUP BY u.id ORDER BY COUNT(*) DESC, u.id LIMIT 5");
    let contributors: Vec<(String, i64)> = qb.build_query_as().fetch_all(db).await?;

    let mut qb = select(user.id);
    push_visible(&mut qb, user.id);
    qb.push(" ORDER BY k.updated_at DESC, k.id DESC LIMIT 6");
    let recent = qb.build_query_as::<KeywordRow>().fetch_all(db).await?;

    // 今日關鍵字：有分享出來的關鍵字裡每天換一個
    let mut qb =
        QueryBuilder::new("SELECT k.id FROM keywords k WHERE k.visibility != 'private' AND ");
    push_visible(&mut qb, user.id);
    qb.push(" ORDER BY k.id");
    let shared: Vec<i64> = qb.build_query_scalar().fetch_all(db).await?;
    let today = if shared.is_empty() {
        None
    } else {
        let day = Utc::now().date_naive().num_days_from_ce() as usize;
        let mut qb = select(user.id);
        qb.push("k.id = ").push_bind(shared[day % shared.len()]);
        let rows = qb.build_query_as::<KeywordRow>().fetch_all(db).await?;
        hydrate(db, &user, rows).await?.pop()
    };

    Ok(Json(json!({
        "total": total,
        "mine": mine,
        "mine_private": mine_private,
        "mine_shared": mine - mine_private,
        "combinations": combinations,
        "favorites": favorites,
        "monthly": monthly,
        "contributors": contributors
            .into_iter()
            .map(|(name, count)| json!({ "name": name, "count": count }))
            .collect::<Vec<_>>(),
        "recent": hydrate(db, &user, recent).await?,
        "today": today,
    })))
}

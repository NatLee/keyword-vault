//! 留言和收藏。

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::keywords::load_one;
use crate::{
    auth::CurrentUser,
    db,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(sqlx::FromRow)]
struct CommentRow {
    id: i64,
    body: String,
    user_id: i64,
    user_name: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
pub struct Comment {
    id: i64,
    body: String,
    user_id: i64,
    user_name: String,
    created_at: String,
    updated_at: String,
    can_edit: bool,
    can_delete: bool,
}

const SELECT: &str =
    "SELECT c.id, c.body, c.user_id, u.display_name AS user_name, c.created_at, c.updated_at
     FROM comments c JOIN users u ON u.id = c.user_id";

fn present(row: CommentRow, user: &CurrentUser) -> Comment {
    Comment {
        can_edit: row.user_id == user.id,
        can_delete: row.user_id == user.id || user.is_admin(),
        id: row.id,
        body: row.body,
        user_id: row.user_id,
        user_name: row.user_name,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn clean_body(body: &str) -> AppResult<String> {
    let body = body.trim();
    if body.is_empty() {
        return Err(AppError::bad("留言不能是空的"));
    }
    if body.chars().count() > 2_000 {
        return Err(AppError::bad("留言最多 2000 個字元"));
    }
    Ok(body.to_owned())
}

pub async fn list(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(keyword_id): Path<i64>,
) -> AppResult<Json<Vec<Comment>>> {
    load_one(&state.db, &user, keyword_id).await?;
    let rows =
        sqlx::query_as::<_, CommentRow>(&format!("{SELECT} WHERE c.keyword_id = ? ORDER BY c.id"))
            .bind(keyword_id)
            .fetch_all(&state.db)
            .await?;
    Ok(Json(rows.into_iter().map(|r| present(r, &user)).collect()))
}

#[derive(Deserialize)]
pub struct CommentBody {
    body: String,
}

async fn one(state: &AppState, user: &CurrentUser, id: i64) -> AppResult<(Comment, i64)> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT keyword_id FROM comments WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let not_found = || AppError::not_found("找不到這則留言");
    let (keyword_id,) = row.ok_or_else(not_found)?;
    // 看不到關鍵字的人也看不到它的留言
    load_one(&state.db, user, keyword_id)
        .await
        .map_err(|_| not_found())?;
    let row = sqlx::query_as::<_, CommentRow>(&format!("{SELECT} WHERE c.id = ?"))
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    Ok((present(row, user), keyword_id))
}

pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(keyword_id): Path<i64>,
    Json(body): Json<CommentBody>,
) -> AppResult<Json<Comment>> {
    load_one(&state.db, &user, keyword_id).await?;
    let now = db::now();
    let id = sqlx::query(
        "INSERT INTO comments (keyword_id, user_id, body, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(keyword_id)
    .bind(user.id)
    .bind(clean_body(&body.body)?)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?
    .last_insert_rowid();
    Ok(Json(one(&state, &user, id).await?.0))
}

pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i64>,
    Json(body): Json<CommentBody>,
) -> AppResult<Json<Comment>> {
    let (comment, _) = one(&state, &user, id).await?;
    if !comment.can_edit {
        return Err(AppError::forbidden("只能修改自己的留言"));
    }
    sqlx::query("UPDATE comments SET body = ?, updated_at = ? WHERE id = ?")
        .bind(clean_body(&body.body)?)
        .bind(db::now())
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(one(&state, &user, id).await?.0))
}

pub async fn remove(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let (comment, _) = one(&state, &user, id).await?;
    if !comment.can_delete {
        return Err(AppError::forbidden("只能刪除自己的留言"));
    }
    sqlx::query("DELETE FROM comments WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn favorite(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(keyword_id): Path<i64>,
) -> AppResult<Json<Value>> {
    load_one(&state.db, &user, keyword_id).await?;
    sqlx::query(
        "INSERT OR IGNORE INTO favorites (user_id, keyword_id, created_at) VALUES (?, ?, ?)",
    )
    .bind(user.id)
    .bind(keyword_id)
    .bind(db::now())
    .execute(&state.db)
    .await?;
    Ok(Json(json!({ "favorite": true })))
}

pub async fn unfavorite(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(keyword_id): Path<i64>,
) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM favorites WHERE user_id = ? AND keyword_id = ?")
        .bind(user.id)
        .bind(keyword_id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "favorite": false })))
}

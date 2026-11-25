//! 帳號和群組的管理（管理員）。

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    auth::{self, Admin, CurrentUser},
    db,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Serialize, sqlx::FromRow)]
pub struct UserRow {
    id: i64,
    username: String,
    display_name: String,
    role: String,
    disabled: bool,
    created_at: String,
    keyword_count: i64,
}

const USER_SELECT: &str =
    "SELECT u.id, u.username, u.display_name, u.role, u.disabled, u.created_at,
        (SELECT COUNT(*) FROM keywords k WHERE k.owner_id = u.id) AS keyword_count
     FROM users u";

pub async fn list(State(state): State<AppState>, _admin: Admin) -> AppResult<Json<Vec<UserRow>>> {
    let users = sqlx::query_as::<_, UserRow>(&format!("{USER_SELECT} ORDER BY u.id"))
        .fetch_all(&state.db)
        .await?;
    Ok(Json(users))
}

async fn one(state: &AppState, id: i64) -> AppResult<UserRow> {
    sqlx::query_as::<_, UserRow>(&format!("{USER_SELECT} WHERE u.id = ?"))
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("找不到這個帳號"))
}

#[derive(Deserialize)]
pub struct CreateUser {
    username: String,
    display_name: Option<String>,
    password: String,
    role: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    _admin: Admin,
    Json(body): Json<CreateUser>,
) -> AppResult<Json<UserRow>> {
    let username = body.username.trim().to_lowercase();
    let display_name = body
        .display_name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| username.clone());
    let role = body.role.as_deref().unwrap_or("member");
    let id = auth::create_user(&state.db, &username, &display_name, &body.password, role).await?;
    Ok(Json(one(&state, id).await?))
}

#[derive(Deserialize)]
pub struct UpdateUser {
    display_name: Option<String>,
    role: Option<String>,
    disabled: Option<bool>,
    password: Option<String>,
}

pub async fn update(
    State(state): State<AppState>,
    Admin(admin): Admin,
    Path(id): Path<i64>,
    Json(body): Json<UpdateUser>,
) -> AppResult<Json<UserRow>> {
    let current = one(&state, id).await?;
    if let Some(name) = &body.display_name {
        let name = auth::clean_display_name(name)?;
        sqlx::query("UPDATE users SET display_name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    let role = body.role.as_deref().unwrap_or(&current.role);
    let disabled = body.disabled.unwrap_or(current.disabled);
    if !matches!(role, "admin" | "member") {
        return Err(AppError::bad("角色只能是 admin 或 member"));
    }
    if role != current.role || disabled != current.disabled {
        if id == admin.id {
            return Err(AppError::bad("不能改自己的角色或停用自己"));
        }
        sqlx::query("UPDATE users SET role = ?, disabled = ? WHERE id = ?")
            .bind(role)
            .bind(disabled)
            .bind(id)
            .execute(&state.db)
            .await?;
        if disabled {
            auth::delete_sessions_of(&state.db, id, None).await?;
        }
    }
    if let Some(password) = &body.password {
        auth::set_password(&state.db, id, password).await?;
        if id != admin.id {
            auth::delete_sessions_of(&state.db, id, None).await?;
        }
    }
    Ok(Json(one(&state, id).await?))
}

#[derive(Serialize)]
pub struct Group {
    id: i64,
    name: String,
    members: Vec<Member>,
}

#[derive(Serialize, Clone)]
pub struct Member {
    id: i64,
    username: String,
    display_name: String,
}

/// 管理員看到全部群組；其他人只看到自己所在的群組。
pub async fn list_groups(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<Vec<Group>>> {
    let groups: Vec<(i64, String)> = if user.is_admin() {
        sqlx::query_as("SELECT id, name FROM user_groups ORDER BY name")
            .fetch_all(&state.db)
            .await?
    } else {
        sqlx::query_as(
            "SELECT g.id, g.name FROM user_groups g JOIN group_members m ON m.group_id = g.id
             WHERE m.user_id = ? ORDER BY g.name",
        )
        .bind(user.id)
        .fetch_all(&state.db)
        .await?
    };
    let rows: Vec<(i64, i64, String, String)> = sqlx::query_as(
        "SELECT m.group_id, u.id, u.username, u.display_name
         FROM group_members m JOIN users u ON u.id = m.user_id ORDER BY u.display_name",
    )
    .fetch_all(&state.db)
    .await?;
    let mut members: HashMap<i64, Vec<Member>> = HashMap::new();
    for (group_id, id, username, display_name) in rows {
        members.entry(group_id).or_default().push(Member {
            id,
            username,
            display_name,
        });
    }
    Ok(Json(
        groups
            .into_iter()
            .map(|(id, name)| Group {
                id,
                name,
                members: members.remove(&id).unwrap_or_default(),
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct GroupBody {
    name: Option<String>,
    member_ids: Option<Vec<i64>>,
}

fn clean_group_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return Err(AppError::bad("群組名稱要 1 到 40 個字元"));
    }
    Ok(name.to_owned())
}

async fn set_members(state: &AppState, group_id: i64, member_ids: &[i64]) -> AppResult<()> {
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM group_members WHERE group_id = ?")
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    let mut ids = member_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    for user_id in ids {
        let added = sqlx::query(
            "INSERT INTO group_members (group_id, user_id) SELECT ?, id FROM users WHERE id = ?",
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        if added.rows_affected() == 0 {
            return Err(AppError::bad(format!("找不到帳號 {user_id}")));
        }
    }
    tx.commit().await?;
    Ok(())
}

pub async fn create_group(
    State(state): State<AppState>,
    _admin: Admin,
    Json(body): Json<GroupBody>,
) -> AppResult<Json<Value>> {
    let name = clean_group_name(body.name.as_deref().unwrap_or_default())?;
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM user_groups WHERE name = ?")
        .bind(&name)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_some() {
        return Err(AppError::conflict("已經有同名的群組"));
    }
    let id = sqlx::query("INSERT INTO user_groups (name, created_at) VALUES (?, ?)")
        .bind(&name)
        .bind(db::now())
        .execute(&state.db)
        .await?
        .last_insert_rowid();
    if let Some(member_ids) = &body.member_ids {
        set_members(&state, id, member_ids).await?;
    }
    Ok(Json(json!({ "id": id, "name": name })))
}

pub async fn update_group(
    State(state): State<AppState>,
    _admin: Admin,
    Path(id): Path<i64>,
    Json(body): Json<GroupBody>,
) -> AppResult<Json<Value>> {
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM user_groups WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_none() {
        return Err(AppError::not_found("找不到這個群組"));
    }
    if let Some(name) = &body.name {
        let name = clean_group_name(name)?;
        let taken: Option<i64> =
            sqlx::query_scalar("SELECT id FROM user_groups WHERE name = ? AND id != ?")
                .bind(&name)
                .bind(id)
                .fetch_optional(&state.db)
                .await?;
        if taken.is_some() {
            return Err(AppError::conflict("已經有同名的群組"));
        }
        sqlx::query("UPDATE user_groups SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(member_ids) = &body.member_ids {
        set_members(&state, id, member_ids).await?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_group(
    State(state): State<AppState>,
    _admin: Admin,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let done = sqlx::query("DELETE FROM user_groups WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::not_found("找不到這個群組"));
    }
    Ok(Json(json!({ "ok": true })))
}

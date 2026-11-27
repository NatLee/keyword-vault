//! Fixed audited operations; domain authorization remains in the original handlers.
use crate::{
    auth::{self, CurrentUser},
    state::{AppState, Inner},
};
use access_kit::{
    AccessConfig, AccessError, AccessKit, CookieSession, DomainOperation, Host, HostFuture,
    HostPaths, Invocation, Principal, Role,
};
use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Weak};
use tower::ServiceExt;

struct VaultHost {
    state: Weak<Inner>,
}
fn unavailable() -> AccessError {
    AccessError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "host_unavailable",
        "Host is unavailable",
    )
}
fn principal(user: CurrentUser) -> Principal {
    Principal {
        user_id: user.id,
        username: user.username,
        display_name: user.display_name,
        role: if user.role == "admin" {
            Role::Admin
        } else {
            Role::Member
        },
    }
}
impl VaultHost {
    fn state(&self) -> Result<AppState, AccessError> {
        self.state.upgrade().ok_or_else(unavailable)
    }
}
pub async fn install(state: &AppState, config: AccessConfig) -> anyhow::Result<()> {
    let kit = AccessKit::new(
        state.db.clone(),
        config,
        Arc::new(VaultHost {
            state: Arc::downgrade(state),
        }),
    )
    .await?;
    state
        .access
        .set(kit)
        .map_err(|_| anyhow::anyhow!("Access core already installed"))?;
    Ok(())
}
fn operation(
    name: &'static str,
    description: &'static str,
    scope: &'static str,
    properties: Value,
    required: Value,
) -> DomainOperation {
    DomainOperation {
        name,
        description,
        required_scope: scope,
        read_only: true,
        input_schema: json!({"type":"object","additionalProperties":false,"properties":properties,"required":required}),
    }
}
impl Host for VaultHost {
    fn registration_admin<'a>(
        &'a self,
        connection: &'a mut sqlx::SqliteConnection,
        headers: &'a HeaderMap,
        id: i64,
    ) -> HostFuture<'a, bool> {
        Box::pin(async move {
            let Some(token) = auth::read_cookie(headers, auth::COOKIE_NAME) else {
                return Ok(false);
            };
            let hash = Sha256::digest(token.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.token_hash=? AND s.user_id=? AND s.expires_at>? AND u.disabled=0 AND u.account_status='active' AND u.role='admin')").bind(hash).bind(id).bind(crate::db::now()).fetch_one(connection).await?)
        })
    }

    fn registration_count<'a>(
        &'a self,
        connection: &'a mut sqlx::SqliteConnection,
    ) -> HostFuture<'a, i64> {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(connection)
                .await?)
        })
    }
    fn registration_create<'a>(
        &'a self,
        connection: &'a mut sqlx::SqliteConnection,
        user: access_kit::RegistrationUser,
    ) -> HostFuture<'a, i64> {
        Box::pin(async move {
            let id=sqlx::query("INSERT INTO users(username,display_name,password_hash,role,created_at,account_status,password_login_enabled) VALUES(?,?,?,'member',?,?,?)")
 .bind(&user.username).bind(&user.display_name).bind(&user.password_hash).bind(crate::db::now()).bind(&user.account_status).bind(user.password_login_enabled).execute(connection).await.map_err(|e| if e.as_database_error().is_some_and(|e|e.is_unique_violation()){AccessError::new(StatusCode::CONFLICT,"username_taken","Username is already in use")}else{unavailable()})?.last_insert_rowid();
            Ok(id)
        })
    }
    fn registration_account<'a>(
        &'a self,
        connection: &'a mut sqlx::SqliteConnection,
        id: i64,
    ) -> HostFuture<'a, Option<access_kit::RegistrationAccount>> {
        Box::pin(async move {
            let row: Option<(String, bool)> = sqlx::query_as(
                "SELECT account_status,password_login_enabled FROM users WHERE id=?",
            )
            .bind(id)
            .fetch_optional(connection)
            .await?;
            Ok(row.map(
                |(status, password_login_enabled)| access_kit::RegistrationAccount {
                    status,
                    password_login_enabled,
                },
            ))
        })
    }
    fn registration_set_status<'a>(
        &'a self,
        connection: &'a mut sqlx::SqliteConnection,
        id: i64,
        status: &'a str,
    ) -> HostFuture<'a, bool> {
        Box::pin(async move {
            Ok(sqlx::query("UPDATE users SET account_status=? WHERE id=? AND account_status='pending' AND role='member'").bind(status).bind(id).execute(connection).await?.rows_affected()==1)
        })
    }

    fn cookie_principal<'a>(
        &'a self,
        headers: &'a HeaderMap,
    ) -> HostFuture<'a, Option<CookieSession>> {
        Box::pin(async move {
            let state = self.state()?;
            let user = auth::optional_user(&state.db, headers)
                .await
                .map_err(|_| unavailable())?;
            Ok(
                match (user, auth::read_cookie(headers, auth::COOKIE_NAME)) {
                    (Some(user), Some(token)) => Some(CookieSession {
                        principal: principal(user),
                        session_binding: Sha256::digest(token.as_bytes()).into(),
                    }),
                    _ => None,
                },
            )
        })
    }
    fn active_principal(&self, id: i64) -> HostFuture<'_, Option<Principal>> {
        Box::pin(async move {
            let state = self.state()?;
            let user = sqlx::query_as::<_, CurrentUser>(
                "SELECT id,username,display_name,role FROM users WHERE id=? AND disabled=0 AND account_status='active'",
            )
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
            Ok(user.map(principal))
        })
    }
    fn create_session(&self, id: i64) -> HostFuture<'_, HeaderValue> {
        Box::pin(async move {
            let state = self.state()?;
            let token = auth::create_session(&state.db, id)
                .await
                .map_err(|_| unavailable())?;
            HeaderValue::from_str(&auth::session_cookie(&token, state.config.cookie_secure))
                .map_err(|_| unavailable())
        })
    }
    fn paths(&self) -> HostPaths {
        HostPaths {
            login: "/login",
            connections: "/connections",
            consent: "/consent",
        }
    }
    fn operations(&self) -> Vec<DomainOperation> {
        let mut registry = vec![
            operation(
                "search_keywords",
                "Search visible owner/public/shared-group keyword records",
                "keywords:read",
                json!({"q":{"type":"string","maxLength":5000},"tag":{"type":"string","maxLength":100},"scope":{"type":"string","enum":["all","mine","shared","favorites"]},"kind":{"type":"string","enum":["all","plain","combination"]},"sort":{"type":"string","enum":["updated","created","theme"]},"page":{"type":"integer","minimum":1,"maximum":1000000},"page_size":{"type":"integer","minimum":1,"maximum":100}}),
                json!([]),
            ),
            operation(
                "get_keyword",
                "Read one currently visible keyword and its existing detail",
                "keywords:read",
                json!({"id":{"type":"integer","minimum":1}}),
                json!(["id"]),
            ),
            operation(
                "inspect_expression",
                "Compile and inspect an expression through the existing Rust parser",
                "expressions:read",
                json!({"expression":{"type":"string","maxLength":50000},"precise":{"type":"boolean"}}),
                json!(["expression"]),
            ),
            operation(
                "inspect_workbench",
                "Inspect and compose source and filter without changing stored expressions",
                "expressions:read",
                json!({"expression":{"type":"string","maxLength":50000},"filter_expression":{"type":"string","maxLength":50000},"precise":{"type":"boolean"},"default_slop":{"type":"integer","minimum":0,"maximum":4294967295_u64}}),
                json!(["expression"]),
            ),
            operation(
                "analyze_expression",
                "Preview Traditional Chinese leaf tokens with a versioned local dictionary; not Elasticsearch analysis",
                "expressions:read",
                json!({"expression":{"type":"string","maxLength":10000},"mode":{"type":"string","enum":["normal","search"]},"precise":{"type":"boolean"}}),
                json!(["expression"]),
            ),
            operation(
                "compare_expressions",
                "Compare expressions with the existing character diff",
                "expressions:read",
                json!({"a":{"type":"string","maxLength":10000},"b":{"type":"string","maxLength":10000}}),
                json!(["a", "b"]),
            ),
        ];
        registry.extend(crate::compiler_compat::operations());
        registry
    }
    fn execute(&self, invocation: Invocation) -> HostFuture<'_, Value> {
        Box::pin(async move {
            let state = self.state()?;
            let args = invocation.arguments();
            let (method, path, body) = match invocation.operation() {
                "search_keywords" => {
                    let params = args
                        .as_object()
                        .ok_or_else(|| {
                            AccessError::new(
                                StatusCode::BAD_REQUEST,
                                "invalid_arguments",
                                "Expected argument object",
                            )
                        })?
                        .iter()
                        .map(|(key, value)| {
                            (
                                key.as_str(),
                                if let Some(v) = value.as_str() {
                                    v.to_owned()
                                } else {
                                    value.to_string()
                                },
                            )
                        })
                        .collect::<Vec<_>>();
                    let query = serde_urlencoded::to_string(params).map_err(|_| unavailable())?;
                    (Method::GET, format!("/keywords?{query}"), Value::Null)
                }
                "get_keyword" => (
                    Method::GET,
                    format!(
                        "/keywords/{}",
                        args["id"].as_i64().ok_or_else(|| AccessError::new(
                            StatusCode::BAD_REQUEST,
                            "invalid_id",
                            "Invalid record ID"
                        ))?
                    ),
                    Value::Null,
                ),
                "compile_query" | "inspect_query" | "extract_terms" => (
                    Method::POST,
                    format!("/compiler/{}", invocation.operation()),
                    args.clone(),
                ),
                "inspect_workbench" => (Method::POST, "/tools/inspect".into(), args.clone()),
                "analyze_expression" => (Method::POST, "/tools/analyze".into(), args.clone()),
                "inspect_expression" => (Method::POST, "/tools/check".into(), args.clone()),
                "compare_expressions" => (Method::POST, "/tools/diff".into(), args.clone()),
                _ => {
                    return Err(AccessError::new(
                        StatusCode::NOT_FOUND,
                        "unknown_operation",
                        "Unknown operation",
                    ))
                }
            };
            let request = Request::builder()
                .method(method)
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .extension(invocation.trusted_principal())
                .body(if body.is_null() {
                    Body::empty()
                } else {
                    Body::from(body.to_string())
                })
                .map_err(|_| unavailable())?;
            let response = crate::routes::access_domain_router(state)
                .oneshot(request)
                .await
                .map_err(|_| unavailable())?;
            let status = response.status();
            let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
                .await
                .map_err(|_| unavailable())?;
            let value: Value = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
            if !status.is_success() {
                return Err(AccessError::new(
                    if matches!(status.as_u16(), 400 | 401 | 403 | 404 | 409 | 429 | 503) {
                        status
                    } else {
                        StatusCode::SERVICE_UNAVAILABLE
                    },
                    if status == StatusCode::BAD_REQUEST
                        && matches!(
                            invocation.operation(),
                            "compile_query" | "inspect_query" | "extract_terms"
                        )
                    {
                        "compile_error"
                    } else {
                        "domain_denied"
                    },
                    value
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("Domain operation failed"),
                ));
            }
            Ok(value)
        })
    }
}

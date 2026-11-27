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
                "SELECT id,username,display_name,role FROM users WHERE id=? AND disabled=0",
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
        vec![
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
                "compare_expressions",
                "Compare expressions with the existing character diff",
                "expressions:read",
                json!({"a":{"type":"string","maxLength":10000},"b":{"type":"string","maxLength":10000}}),
                json!(["a", "b"]),
            ),
        ]
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
                    "domain_denied",
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

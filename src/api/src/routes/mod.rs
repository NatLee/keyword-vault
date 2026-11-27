//! HTTP API（規格見 `docs/api.md`）和前端靜態檔。

mod auth;
mod comments;
pub mod keywords;
mod stats;
mod tools;
mod transfer;
mod users;

use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderValue},
    middleware,
    routing::{get, patch, post, put},
    Json, Router,
};
use serde_json::{json, Value};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

use crate::{auth::origin_guard, error::AppError, state::AppState};

/// 匯入的 CSV 大小上限。
const MAX_IMPORT_BYTES: usize = 5 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/meta", get(meta))
        .route("/auth/state", get(auth::state))
        .route("/auth/setup", post(auth::setup))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", patch(auth::update_me))
        .route("/auth/password", post(auth::change_password))
        .route("/users", get(users::list).post(users::create))
        .route("/users/{id}", patch(users::update))
        .route("/groups", get(users::list_groups).post(users::create_group))
        .route(
            "/groups/{id}",
            patch(users::update_group).delete(users::delete_group),
        )
        .route("/keywords", get(keywords::list).post(keywords::create))
        .route("/keywords/random", get(keywords::random))
        .route(
            "/keywords/import",
            post(transfer::import).layer(DefaultBodyLimit::max(MAX_IMPORT_BYTES)),
        )
        .route("/keywords/export", get(transfer::export))
        .route(
            "/keywords/{id}",
            get(keywords::detail)
                .patch(keywords::update)
                .delete(keywords::remove),
        )
        .route("/keywords/{id}/history", get(keywords::history))
        .route(
            "/keywords/{id}/favorite",
            put(comments::favorite).delete(comments::unfavorite),
        )
        .route(
            "/keywords/{id}/comments",
            get(comments::list).post(comments::create),
        )
        .route(
            "/comments/{id}",
            patch(comments::update).delete(comments::remove),
        )
        .route("/tags", get(stats::tags))
        .route("/stats", get(stats::overview))
        .route("/tools/check", post(tools::check))
        .route("/tools/inspect", post(tools::inspect))
        .route("/compiler/inspect", post(crate::compiler_compat::inspect))
        .route("/tools/diff", post(tools::diff))
        .fallback(|| async { AppError::not_found("找不到這個 API") });

    // 前端是單頁應用：找不到的路徑一律回 index.html，交給前端路由。
    let static_dir = state.config.static_dir.clone();
    let spa = ServeDir::new(&static_dir).fallback(ServeFile::new(static_dir.join("index.html")));

    Router::new()
        .nest("/api", api)
        // 同源檢查只套在 API 上
        .route_layer(middleware::from_fn_with_state(state.clone(), origin_guard))
        .fallback_service(spa)
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("same-origin"),
        ))
        .layer(TraceLayer::new_for_http().make_span_with(
            |request: &axum::http::Request<axum::body::Body>| {
                let route = request
                    .extensions()
                    .get::<axum::extract::MatchedPath>()
                    .map(|path| path.as_str())
                    .unwrap_or("static");
                tracing::info_span!("http_request",method=%request.method(),route)
            },
        ))
        .with_state(state)
}

async fn meta(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "default_slop": state.config.default_slop,
    }))
}

/// Uncounted in-process routes, reachable only through the fixed access registry.
pub(crate) fn access_domain_router(state: AppState) -> Router {
    Router::new()
        .route("/keywords", get(keywords::list))
        .route("/keywords/{id}", get(keywords::detail))
        .route("/tools/check", post(tools::check))
        .route("/tools/inspect", post(tools::inspect))
        .route(
            "/compiler/{operation}",
            post(crate::compiler_compat::invoke),
        )
        .route("/tools/diff", post(tools::diff))
        .with_state(state)
}

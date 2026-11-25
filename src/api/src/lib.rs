//! 關鍵字庫：撰寫、檢查、保存和分享布林搜尋關鍵字，
//! 並把它編譯成 Elasticsearch / OpenSearch 的 `query_string`。

pub mod auth;
pub mod combo;
pub mod config;
pub mod db;
pub mod demo;
pub mod error;
pub mod routes;
pub mod state;
pub mod syntax;

use std::sync::Arc;

use anyhow::Result;
use axum::Router;

pub use config::Config;
pub use state::{AppState, Inner};

/// 開資料庫（必要時建立並套用 migration）。
pub async fn build_state(config: Config) -> Result<AppState> {
    let db = db::connect(&config.database_path).await?;
    Ok(Arc::new(Inner::new(db, config)))
}

pub fn app(state: AppState) -> Router {
    routes::router(state)
}

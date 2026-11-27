//! 關鍵字庫：撰寫、檢查、保存和分享布林搜尋關鍵字，
//! 並把它編譯成 Elasticsearch / OpenSearch 的 `query_string`。

pub mod access;
pub mod analysis;
pub mod auth;
pub mod combo;
pub mod compiler_compat;
pub mod config;
pub mod db;
pub mod demo;
pub mod error;
pub mod migration;
pub mod routes;
pub mod setup;
pub mod state;
pub mod syntax;
pub mod workbench;

use std::sync::Arc;

use anyhow::Result;
use axum::Router;

pub use config::Config;
pub use state::{AppState, Inner};

/// 開資料庫（必要時建立並套用 migration）。
pub async fn build_state(mut config: Config) -> Result<AppState> {
    let origin = std::env::var("ACCESS_PUBLIC_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| {
            config
                .public_origin
                .clone()
                .unwrap_or_else(|| "http://127.0.0.1:8686".into())
        });
    let access_config = access_kit::AccessConfig::from_env(&origin)?;
    config.cookie_secure |= access_config.secure_cookies();
    config.public_origin = Some(url::Url::parse(&origin)?.origin().ascii_serialization());
    let analyzer = Arc::new(analysis::Analyzer::load(
        config.segmentation_dict.as_deref(),
    )?);
    let db = db::connect(&config.database_path).await?;
    let state = Arc::new(Inner::new(db, config, analyzer));
    access::install(&state, access_config).await?;
    Ok(state)
}

pub fn app(state: AppState) -> Router {
    let router = routes::router(state.clone());
    match state.access.get() {
        Some(kit) => router.merge(kit.router()),
        None => router,
    }
}

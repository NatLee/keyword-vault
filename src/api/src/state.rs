use std::sync::{Arc, OnceLock};

use sqlx::SqlitePool;

use crate::{auth::LoginLimiter, config::Config};

pub type AppState = Arc<Inner>;

pub struct Inner {
    pub db: SqlitePool,
    pub config: Config,
    pub limiter: LoginLimiter,
    pub access: OnceLock<access_kit::AccessKit>,
    pub analyzer: Arc<crate::analysis::Analyzer>,
    pub analysis_slots: Arc<tokio::sync::Semaphore>,
}

impl Inner {
    pub fn new(db: SqlitePool, config: Config, analyzer: Arc<crate::analysis::Analyzer>) -> Self {
        Self {
            db,
            config,
            limiter: LoginLimiter::default(),
            access: OnceLock::new(),
            analyzer,
            analysis_slots: Arc::new(tokio::sync::Semaphore::new(2)),
        }
    }
}

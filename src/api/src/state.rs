use std::sync::{Arc, OnceLock};

use sqlx::SqlitePool;

use crate::{auth::LoginLimiter, config::Config};

pub type AppState = Arc<Inner>;

pub struct Inner {
    pub db: SqlitePool,
    pub config: Config,
    pub limiter: LoginLimiter,
    pub access: OnceLock<access_kit::AccessKit>,
}

impl Inner {
    pub fn new(db: SqlitePool, config: Config) -> Self {
        Self {
            db,
            config,
            limiter: LoginLimiter::default(),
            access: OnceLock::new(),
        }
    }
}

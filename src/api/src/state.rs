use std::sync::Arc;

use sqlx::SqlitePool;

use crate::{auth::LoginLimiter, config::Config};

pub type AppState = Arc<Inner>;

pub struct Inner {
    pub db: SqlitePool,
    pub config: Config,
    pub limiter: LoginLimiter,
}

impl Inner {
    pub fn new(db: SqlitePool, config: Config) -> Self {
        Self {
            db,
            config,
            limiter: LoginLimiter::default(),
        }
    }
}

//! First-account setup is authorized by the real peer or a deployment credential.
use crate::{
    auth::{self, CurrentUser},
    error::{AppError, AppResult},
};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct SetupToken(String);
impl std::fmt::Debug for SetupToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}
impl SetupToken {
    pub fn new(token: String) -> anyhow::Result<Self> {
        if !(32..=512).contains(&token.chars().count()) || token.trim().is_empty() {
            anyhow::bail!("ACCESS_SETUP_TOKEN must contain 32 to 512 characters");
        }
        Ok(Self(token))
    }
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        match std::env::var("ACCESS_SETUP_TOKEN") {
            Err(std::env::VarError::NotPresent) => Ok(None),
            Ok(value) => Self::new(value).map(Some),
            Err(_) => anyhow::bail!("ACCESS_SETUP_TOKEN must be valid Unicode"),
        }
    }
    pub fn matches(&self, candidate: &str) -> bool {
        bool::from(Sha256::digest(self.0.as_bytes()).ct_eq(&Sha256::digest(candidate.as_bytes())))
    }
}

pub type Peer = Option<axum::Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>;
pub fn is_loopback(peer: &Peer) -> bool {
    peer.as_ref().is_some_and(|p| p.0 .0.ip().is_loopback())
}

pub async fn create_admin(
    db: &SqlitePool,
    username: &str,
    display_name: Option<String>,
    password: String,
) -> AppResult<CurrentUser> {
    let username = username.trim().to_lowercase();
    auth::validate_username(&username)?;
    auth::validate_password(&password)?;
    let display_name = auth::clean_display_name(
        &display_name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| username.clone()),
    )?;
    let hash = auth::hash_password(password).await?;
    let inserted=sqlx::query("INSERT INTO users (username, display_name, password_hash, role, created_at) SELECT ?, ?, ?, 'admin', ? WHERE NOT EXISTS (SELECT 1 FROM users)")
        .bind(&username).bind(&display_name).bind(hash).bind(crate::db::now()).execute(db).await?;
    if inserted.rows_affected() == 0 {
        return Err(AppError::conflict("已經設定過了，請直接登入"));
    }
    Ok(CurrentUser {
        id: inserted.last_insert_rowid(),
        username,
        display_name,
        role: "admin".into(),
    })
}

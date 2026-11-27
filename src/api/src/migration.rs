//! Offline account import. Reports contain plans and fingerprints, never secrets.
use anyhow::{bail, Context, Result};
use argon2::password_hash::PasswordHash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha384};
use sqlx::{sqlite::SqliteConnectOptions, Connection, Row, SqliteConnection};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const SOURCE_MIGRATIONS: [&[u8]; 4] = [
    include_bytes!("../compiler-migrations/0001_host.sql"),
    include_bytes!("../compiler-migrations/0002_access.sql"),
    include_bytes!("../compiler-migrations/0003_access_client_registration.sql"),
    include_bytes!("../compiler-migrations/0004_registration.sql"),
];
const DESTINATION_MIGRATIONS: [&[u8]; 4] = [
    include_bytes!("../migrations/0001_init.sql"),
    include_bytes!("../migrations/0002_access.sql"),
    include_bytes!("../migrations/0003_access_client_registration.sql"),
    include_bytes!("../migrations/0004_registration.sql"),
];
const EXCLUDED: [&str; 12] = [
    "sessions",
    "access_credentials",
    "access_clients",
    "access_consents",
    "access_codes",
    "access_grants",
    "access_tokens",
    "access_provider_flows",
    "access_events",
    "access_daily",
    "access_registration_events",
    "access_registration_policy",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseFingerprint {
    pub path: PathBuf,
    pub file_sha256: String,
    pub schema_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserPlan {
    pub source_id: i64,
    pub target_id: i64,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub disabled: i64,
    pub account_status: String,
    pub password_login_enabled: i64,
    pub password_format: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityPlan {
    pub source_id: String,
    pub target_id: String,
    pub user_id: i64,
    pub provider: String,
    pub subject: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationPlan {
    pub source_id: i64,
    pub target_id: i64,
    pub user_id: i64,
    pub reviewed_by: Option<i64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationReport {
    pub schema_version: u32,
    pub source: DatabaseFingerprint,
    pub destination: DatabaseFingerprint,
    pub users: Vec<UserPlan>,
    pub identities: Vec<IdentityPlan>,
    pub registrations: Vec<RegistrationPlan>,
    pub conflicts: Vec<String>,
    pub excluded: BTreeMap<String, i64>,
}
#[derive(Debug, Serialize)]
pub struct MigrationReceipt {
    pub imported_users: usize,
    pub user_ids: BTreeMap<i64, i64>,
    pub backup: DatabaseFingerprint,
}

#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
struct UserRow {
    id: i64,
    username: String,
    display_name: String,
    password_hash: String,
    role: String,
    disabled: i64,
    created_at: String,
    account_status: String,
    password_login_enabled: i64,
}
#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
struct IdentityRow {
    id: String,
    user_id: i64,
    provider: String,
    subject: String,
    linked_at: i64,
    last_login_at: Option<i64>,
}
#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
struct RegistrationRow {
    id: i64,
    user_id: i64,
    username: String,
    display_name: String,
    provider: String,
    status: String,
    created_at: i64,
    reviewed_at: Option<i64>,
    reviewed_by: Option<i64>,
    rejection_reason: Option<String>,
}

fn file_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path).context("Cannot read selected database")?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        hash.update(&buffer[..size]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn offline(path: &Path) -> Result<()> {
    if !path.is_file() {
        bail!("Selected database must be an existing regular file");
    }
    let mut wal = path.as_os_str().to_os_string();
    wal.push("-wal");
    if Path::new(&wal).try_exists()? && std::fs::metadata(&wal)?.len() > 0 {
        bail!("Database has outstanding WAL data; stop the service and checkpoint/close it first");
    }
    Ok(())
}
async fn read_connection(path: &Path) -> Result<SqliteConnection> {
    offline(path)?;
    Ok(SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(path)
            .read_only(true)
            .immutable(true)
            .create_if_missing(false),
    )
    .await?)
}
#[derive(Clone, Copy)]
enum DatabaseKind {
    Compiler,
    Vault,
}

async fn check_schema(conn: &mut SqliteConnection, kind: DatabaseKind) -> Result<String> {
    let expected = match kind {
        DatabaseKind::Compiler => &SOURCE_MIGRATIONS,
        DatabaseKind::Vault => &DESTINATION_MIGRATIONS,
    };
    let migrations: Vec<(i64, Vec<u8>, bool)> =
        sqlx::query_as("SELECT version,checksum,success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&mut *conn)
            .await
            .context("Not a supported SQLx database")?;
    if migrations.len() != expected.len() {
        bail!("Unsupported database migration version");
    }
    for (index, (version, checksum, success)) in migrations.iter().enumerate() {
        if *version != (index + 1) as i64
            || !success
            || checksum.as_slice() != Sha384::digest(expected[index]).as_slice()
        {
            bail!("Unsupported migration fingerprint at version {version}");
        }
    }
    let integrity: Vec<(String,)> = sqlx::query_as("PRAGMA integrity_check")
        .fetch_all(&mut *conn)
        .await?;
    if integrity != vec![("ok".to_owned(),)] {
        bail!("Database integrity check failed");
    }
    if !sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&mut *conn)
        .await?
        .is_empty()
    {
        bail!("Database foreign-key check failed");
    }
    // Migration receipts alone cannot establish the actual layout: extra triggers
    // could change account restrictions during INSERT. Build the supported layout
    // from the same embedded migrations in an isolated in-memory database.
    let actual = schema_hash(conn).await?;
    let mut canonical = SqliteConnection::connect("sqlite::memory:").await?;
    match kind {
        DatabaseKind::Compiler => {
            sqlx::migrate!("./compiler-migrations")
                .run(&mut canonical)
                .await?
        }
        DatabaseKind::Vault => sqlx::migrate!("./migrations").run(&mut canonical).await?,
    }
    let supported = schema_hash(&mut canonical).await?;
    canonical.close().await?;
    if actual != supported {
        bail!("Unsupported actual database schema; tables, indexes and triggers must match the supported migrations");
    }
    Ok(actual)
}

async fn schema_hash(conn: &mut SqliteConnection) -> Result<String> {
    let rows=sqlx::query("SELECT type,name,tbl_name,COALESCE(sql,'') AS definition FROM sqlite_schema ORDER BY type,name").fetch_all(&mut *conn).await?;
    let mut hash = Sha256::new();
    for row in rows {
        for column in ["type", "name", "tbl_name", "definition"] {
            hash.update(row.try_get::<String, _>(column)?.as_bytes());
            hash.update([0]);
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}
async fn users(conn: &mut SqliteConnection) -> Result<Vec<UserRow>> {
    Ok(sqlx::query_as("SELECT id,username,display_name,password_hash,role,disabled,created_at,account_status,password_login_enabled FROM users ORDER BY id").fetch_all(conn).await?)
}
async fn identities(conn: &mut SqliteConnection) -> Result<Vec<IdentityRow>> {
    Ok(sqlx::query_as("SELECT id,user_id,provider,subject,linked_at,last_login_at FROM access_identities ORDER BY id").fetch_all(conn).await?)
}
async fn registrations(conn: &mut SqliteConnection) -> Result<Vec<RegistrationRow>> {
    Ok(sqlx::query_as("SELECT id,user_id,username,display_name,provider,status,created_at,reviewed_at,reviewed_by,rejection_reason FROM access_registrations ORDER BY id").fetch_all(conn).await?)
}

// Compare complete account rows, including existing accounts and authentication
// identities, before commit. Hashes remain internal and are never serialized.
#[derive(PartialEq, Eq)]
struct AccountSnapshot {
    users: Vec<UserRow>,
    identities: Vec<IdentityRow>,
    registrations: Vec<RegistrationRow>,
}
impl AccountSnapshot {
    async fn load(conn: &mut SqliteConnection) -> Result<Self> {
        Ok(Self {
            users: users(conn).await?,
            identities: identities(conn).await?,
            registrations: registrations(conn).await?,
        })
    }
    fn include_import(
        &mut self,
        source_users: &[UserRow],
        source_identities: &[IdentityRow],
        source_registrations: &[RegistrationRow],
        report: &MigrationReport,
    ) {
        self.users.extend(
            source_users
                .iter()
                .zip(&report.users)
                .map(|(row, plan)| UserRow {
                    id: plan.target_id,
                    ..row.clone()
                }),
        );
        self.identities
            .extend(
                source_identities
                    .iter()
                    .zip(&report.identities)
                    .map(|(row, plan)| IdentityRow {
                        id: plan.target_id.clone(),
                        user_id: plan.user_id,
                        ..row.clone()
                    }),
            );
        self.registrations
            .extend(
                source_registrations
                    .iter()
                    .zip(&report.registrations)
                    .map(|(row, plan)| RegistrationRow {
                        id: plan.target_id,
                        user_id: plan.user_id,
                        reviewed_by: plan.reviewed_by,
                        ..row.clone()
                    }),
            );
        self.users.sort_by_key(|row| row.id);
        self.identities.sort_by(|a, b| a.id.cmp(&b.id));
        self.registrations.sort_by_key(|row| row.id);
    }
    async fn verify(&self, conn: &mut SqliteConnection) -> Result<()> {
        if &Self::load(conn).await? != self {
            bail!("Account preservation verification failed; all account changes rolled back");
        }
        Ok(())
    }
}

fn password_format(user: &UserRow) -> Result<String> {
    if user.password_login_enabled == 0 {
        return Ok("disabled".into());
    }
    let hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| anyhow::anyhow!("unsupported password hash"))?;
    let (m, t, p) = (
        hash.params.get_decimal("m"),
        hash.params.get_decimal("t"),
        hash.params.get_decimal("p"),
    );
    let Some((m, t, p)) = m.zip(t).zip(p).map(|((m, t), p)| (m, t, p)) else {
        bail!("unsupported password hash parameters");
    };
    if hash.algorithm.as_str() != "argon2id"
        || hash.version != Some(19)
        || hash.params.iter().count() != 3
        || !(8192..=65536).contains(&m)
        || !(1..=6).contains(&t)
        || !(1..=4).contains(&p)
        || hash.hash.is_none_or(|v| !(16..=64).contains(&v.len()))
        || hash.salt.is_none()
    {
        bail!("unsupported password hash parameters");
    }
    Ok(format!("argon2id-v19;m={m},t={t},p={p}"))
}

pub async fn inspect(source: &Path, destination: &Path) -> Result<MigrationReport> {
    let source = source
        .canonicalize()
        .context("Source database does not exist")?;
    let destination = destination
        .canonicalize()
        .context("Destination database does not exist")?;
    if source == destination {
        bail!("Source and destination must be different databases");
    }
    let mut src = read_connection(&source).await?;
    let mut dst = read_connection(&destination).await?;
    let source_hash = file_hash(&source)?;
    let destination_hash = file_hash(&destination)?;
    let source_schema = check_schema(&mut src, DatabaseKind::Compiler).await?;
    let destination_schema = check_schema(&mut dst, DatabaseKind::Vault).await?;
    let source_users = users(&mut src).await?;
    let target_users = users(&mut dst).await?;
    let source_identities = identities(&mut src).await?;
    let target_identities = identities(&mut dst).await?;
    let source_registrations = registrations(&mut src).await?;
    let target_names: HashSet<_> = target_users
        .iter()
        .map(|u| u.username.to_ascii_lowercase())
        .collect();
    let target_subjects: HashSet<_> = target_identities
        .iter()
        .map(|i| (i.provider.as_str(), i.subject.as_str()))
        .collect();
    let target_identity_ids: HashSet<_> = target_identities.iter().map(|i| i.id.as_str()).collect();
    let source_by_id: HashMap<_, _> = source_users.iter().map(|u| (u.id, u)).collect();
    let mut next = target_users.iter().map(|u| u.id).max().unwrap_or(0).max(0);
    let mut conflicts = Vec::new();
    let mut planned = Vec::new();
    let mut id_map = HashMap::new();
    for user in &source_users {
        next = next
            .checked_add(1)
            .context("Destination user IDs exhausted")?;
        id_map.insert(user.id, next);
        if target_names.contains(&user.username.to_ascii_lowercase()) {
            conflicts.push(format!(
                "User {}: username conflicts with an existing account",
                user.id
            ));
        }
        if user.id <= 0
            || !matches!(user.role.as_str(), "admin" | "member")
            || !matches!(
                user.account_status.as_str(),
                "active" | "pending" | "rejected"
            )
            || ![0, 1].contains(&user.disabled)
            || ![0, 1].contains(&user.password_login_enabled)
        {
            conflicts.push(format!("User {}: unsupported account state", user.id));
        }
        if user.account_status != "active"
            && !source_registrations
                .iter()
                .any(|r| r.user_id == user.id && r.status == user.account_status)
        {
            conflicts.push(format!(
                "User {}: registration state is missing or inconsistent",
                user.id
            ));
        }
        if user.password_login_enabled == 0
            && !source_identities.iter().any(|i| i.user_id == user.id)
        {
            conflicts.push(format!("User {}: no usable login method", user.id));
        }
        let format = match password_format(user) {
            Ok(value) => value,
            Err(error) => {
                conflicts.push(format!("User {}: {error}", user.id));
                "unsupported".into()
            }
        };
        planned.push(UserPlan {
            source_id: user.id,
            target_id: next,
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            role: user.role.clone(),
            disabled: user.disabled,
            account_status: user.account_status.clone(),
            password_login_enabled: user.password_login_enabled,
            password_format: format,
        });
    }
    let mut identity_plans = Vec::new();
    for identity in &source_identities {
        let Some(&user_id) = id_map.get(&identity.user_id) else {
            conflicts.push(format!("Identity {}: missing source user", identity.id));
            continue;
        };
        if !matches!(identity.provider.as_str(), "google" | "github")
            || target_subjects.contains(&(identity.provider.as_str(), identity.subject.as_str()))
        {
            conflicts.push(format!(
                "Identity {}: unsupported provider or existing provider subject",
                identity.id
            ));
        }
        let target_id = format!(
            "import-{}-{:x}",
            &source_hash[..16],
            Sha256::digest(identity.id.as_bytes())
        );
        if target_identity_ids.contains(target_id.as_str()) {
            conflicts.push(format!(
                "Identity {}: generated ID already exists",
                identity.id
            ));
        }
        identity_plans.push(IdentityPlan {
            source_id: identity.id.clone(),
            target_id,
            user_id,
            provider: identity.provider.clone(),
            subject: identity.subject.clone(),
        });
    }
    let mut registration_id: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(id),0) FROM access_registrations")
            .fetch_one(&mut dst)
            .await?;
    let mut registration_plans = Vec::new();
    for registration in &source_registrations {
        let Some(&user_id) = id_map.get(&registration.user_id) else {
            conflicts.push(format!(
                "Registration {}: missing source user",
                registration.id
            ));
            continue;
        };
        if !matches!(
            registration.provider.as_str(),
            "password" | "google" | "github"
        ) || source_by_id[&registration.user_id].account_status != registration.status
        {
            conflicts.push(format!(
                "Registration {}: inconsistent account state",
                registration.id
            ));
        }
        let reviewer = registration
            .reviewed_by
            .and_then(|id| id_map.get(&id).copied());
        if registration.reviewed_by.is_some() && reviewer.is_none() {
            conflicts.push(format!(
                "Registration {}: missing reviewer",
                registration.id
            ));
        }
        registration_id = registration_id
            .checked_add(1)
            .context("Registration IDs exhausted")?;
        registration_plans.push(RegistrationPlan {
            source_id: registration.id,
            target_id: registration_id,
            user_id,
            reviewed_by: reviewer,
        });
    }
    let mut excluded = BTreeMap::new();
    for table in EXCLUDED {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&mut src)
            .await?;
        excluded.insert(table.into(), count);
    }
    src.close().await?;
    dst.close().await?;
    offline(&source)?;
    offline(&destination)?;
    if file_hash(&source)? != source_hash || file_hash(&destination)? != destination_hash {
        bail!("Database changed during dry-run; stop services and try again");
    }
    Ok(MigrationReport {
        schema_version: 1,
        source: DatabaseFingerprint {
            path: source,
            file_sha256: source_hash,
            schema_sha256: source_schema,
        },
        destination: DatabaseFingerprint {
            path: destination,
            file_sha256: destination_hash,
            schema_sha256: destination_schema,
        },
        users: planned,
        identities: identity_plans,
        registrations: registration_plans,
        conflicts,
        excluded,
    })
}

fn copy_backup(source: &Path, backup: &Path, expected: &str) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut out = options
        .open(backup)
        .context("Backup must be a new file; existing files are never overwritten")?;
    std::io::copy(&mut File::open(source)?, &mut out)?;
    out.flush()?;
    out.sync_all()?;
    if file_hash(backup)? != expected {
        bail!("Backup verification failed; no accounts imported");
    }
    Ok(())
}

pub async fn apply(report: &MigrationReport, backup: &Path) -> Result<MigrationReceipt> {
    if report.schema_version != 1 || !report.conflicts.is_empty() {
        bail!("Migration report has conflicts or an unsupported version");
    }
    let fresh = inspect(&report.source.path, &report.destination.path).await?;
    if &fresh != report {
        bail!("Migration report or database fingerprints changed; generate a new dry-run report");
    }
    let mut destination = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(&report.destination.path)
            .create_if_missing(false)
            .busy_timeout(Duration::from_secs(2)),
    )
    .await?;
    let mut tx = destination
        .begin_with("BEGIN EXCLUSIVE")
        .await
        .context("Destination is busy; stop the service first")?;
    if inspect(&report.source.path, &report.destination.path).await? != *report {
        bail!("Database changed before apply; generate a new report");
    }
    copy_backup(
        &report.destination.path,
        backup,
        &report.destination.file_sha256,
    )?;
    let backup_path = backup.canonicalize()?;
    let mut backup_conn = read_connection(&backup_path).await?;
    let backup_schema = check_schema(&mut backup_conn, DatabaseKind::Vault).await?;
    backup_conn.close().await?;
    if backup_schema != report.destination.schema_sha256 {
        bail!("Backup schema verification failed");
    }
    let mut source = read_connection(&report.source.path).await?;
    let source_users = users(&mut source).await?;
    let source_identities = identities(&mut source).await?;
    let source_registrations = registrations(&mut source).await?;
    let mut expected_accounts = AccountSnapshot::load(&mut tx).await?;
    expected_accounts.include_import(
        &source_users,
        &source_identities,
        &source_registrations,
        report,
    );
    for (user, plan) in source_users.iter().zip(&report.users) {
        sqlx::query("INSERT INTO users(id,username,display_name,password_hash,role,disabled,created_at,account_status,password_login_enabled)VALUES(?,?,?,?,?,?,?,?,?)")
            .bind(plan.target_id).bind(&user.username).bind(&user.display_name).bind(&user.password_hash).bind(&user.role).bind(user.disabled).bind(&user.created_at).bind(&user.account_status).bind(user.password_login_enabled).execute(&mut *tx).await.context("Account insertion failed; all account changes rolled back")?;
    }
    for (identity, plan) in source_identities.iter().zip(&report.identities) {
        sqlx::query("INSERT INTO access_identities(id,user_id,provider,subject,linked_at,last_login_at)VALUES(?,?,?,?,?,?)")
            .bind(&plan.target_id).bind(plan.user_id).bind(&identity.provider).bind(&identity.subject).bind(identity.linked_at).bind(identity.last_login_at).execute(&mut *tx).await?;
    }
    for (registration, plan) in source_registrations.iter().zip(&report.registrations) {
        sqlx::query("INSERT INTO access_registrations(id,user_id,username,display_name,provider,status,created_at,reviewed_at,reviewed_by,rejection_reason)VALUES(?,?,?,?,?,?,?,?,?,?)")
            .bind(plan.target_id).bind(plan.user_id).bind(&registration.username).bind(&registration.display_name).bind(&registration.provider).bind(&registration.status).bind(registration.created_at).bind(registration.reviewed_at).bind(plan.reviewed_by).bind(&registration.rejection_reason).execute(&mut *tx).await?;
    }
    source.close().await?;
    offline(&report.source.path)?;
    if file_hash(&report.source.path)? != report.source.file_sha256 {
        bail!("Source changed while importing; all account changes rolled back");
    }
    commit_preserving_accounts(tx, &expected_accounts).await?;
    destination.close().await?;
    Ok(MigrationReceipt {
        imported_users: report.users.len(),
        user_ids: report
            .users
            .iter()
            .map(|p| (p.source_id, p.target_id))
            .collect(),
        backup: DatabaseFingerprint {
            path: backup_path,
            file_sha256: report.destination.file_sha256.clone(),
            schema_sha256: backup_schema,
        },
    })
}

pub fn write_report(path: &Path, report: &MigrationReport) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .context("Report path must be a new file")?;
    serde_json::to_writer_pretty(&mut file, report)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

async fn commit_preserving_accounts(
    mut tx: sqlx::Transaction<'_, sqlx::Sqlite>,
    expected: &AccountSnapshot,
) -> Result<()> {
    expected.verify(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn final_account_verification_rolls_back_every_restriction_and_identity_change() {
        let mut conn = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&mut conn).await.unwrap();
        sqlx::raw_sql("INSERT INTO users(id,username,display_name,password_hash,role,disabled,created_at,account_status,password_login_enabled) VALUES(2,'existing','Existing','preserved-hash','admin',0,'today','active',1),(101,'imported','Imported','','member',1,'today','pending',0); INSERT INTO access_identities(id,user_id,provider,subject,linked_at) VALUES('mapped',101,'google','original-subject',1); INSERT INTO access_registrations(id,user_id,username,display_name,provider,status,created_at,reviewed_by) VALUES(29,101,'imported','Imported','google','pending',1,2);").execute(&mut conn).await.unwrap();
        let expected = AccountSnapshot::load(&mut conn).await.unwrap();
        for mutation in [
            "UPDATE users SET disabled=0 WHERE id=101",
            "UPDATE users SET role='admin' WHERE id=101",
            "UPDATE users SET account_status='active' WHERE id=101",
            "UPDATE users SET password_login_enabled=1 WHERE id=101",
            "UPDATE users SET password_hash='changed' WHERE id=101",
            "UPDATE users SET disabled=1 WHERE id=2",
            "UPDATE access_identities SET user_id=2 WHERE id='mapped'",
            "UPDATE access_identities SET provider='github' WHERE id='mapped'",
            "UPDATE access_identities SET subject='changed' WHERE id='mapped'",
            "UPDATE access_registrations SET status='active' WHERE id=29",
            "UPDATE access_registrations SET reviewed_by=NULL WHERE id=29",
        ] {
            let mut tx = conn.begin().await.unwrap();
            sqlx::query(mutation).execute(&mut *tx).await.unwrap();
            assert!(
                commit_preserving_accounts(tx, &expected).await.is_err(),
                "mutation was committed: {mutation}"
            );
            expected.verify(&mut conn).await.unwrap();
        }
        let tx = conn.begin().await.unwrap();
        commit_preserving_accounts(tx, &expected).await.unwrap();
        conn.close().await.unwrap();
    }
}

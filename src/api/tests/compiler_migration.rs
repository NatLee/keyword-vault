use keyword_vault::{auth, db, migration};
use sqlx::{sqlite::SqliteConnectOptions, Connection, SqliteConnection};
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Fixture {
    _dir: tempfile::TempDir,
    source: PathBuf,
    destination: PathBuf,
}

async fn open(path: &Path) -> SqliteConnection {
    SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true),
    )
    .await
    .unwrap()
}
async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("compiler.db");
    let destination = dir.path().join("vault.db");
    let mut src = open(&source).await;
    sqlx::migrate!("./compiler-migrations")
        .run(&mut src)
        .await
        .unwrap();
    let hash = auth::hash_password("migration-password-123".into())
        .await
        .unwrap();
    for (id, name, role, status, disabled, password_enabled) in [
        (11, "admin", "admin", "active", 0, 1),
        (12, "member", "member", "active", 0, 1),
        (13, "pending", "member", "pending", 0, 1),
        (14, "rejected", "member", "rejected", 0, 1),
        (15, "disabled", "member", "active", 1, 1),
        (16, "provider", "member", "active", 0, 0),
    ] {
        sqlx::query("INSERT INTO users(id,username,display_name,password_hash,role,disabled,created_at,account_status,password_login_enabled)VALUES(?,?,?,?,?,?,?,?,?)").bind(id).bind(name).bind(name).bind(if password_enabled==1{hash.as_str()}else{""}).bind(role).bind(disabled).bind("2026-11-28T00:00:00Z").bind(status).bind(password_enabled).execute(&mut src).await.unwrap();
    }
    for (id, user, status, reviewer) in [(1, 13, "pending", None), (2, 14, "rejected", Some(11))] {
        sqlx::query("INSERT INTO access_registrations(id,user_id,username,display_name,provider,status,created_at,reviewed_by,rejection_reason)VALUES(?,?,?,?,?,?,?,?,?)").bind(id).bind(user).bind(status).bind(status).bind("password").bind(status).bind(100).bind(reviewer).bind(if status=="rejected"{Some("not approved")}else{None}).execute(&mut src).await.unwrap();
    }
    sqlx::query("INSERT INTO access_identities(id,user_id,provider,subject,linked_at)VALUES('old-provider',16,'google','source-google-subject',100)").execute(&mut src).await.unwrap();
    sqlx::query("INSERT INTO sessions(token_hash,user_id,expires_at,created_at)VALUES('secret-session-canary',11,'2099-01-01','2026-01-01')").execute(&mut src).await.unwrap();
    sqlx::query("INSERT INTO access_credentials(id,user_id,name,prefix,token_hash,scopes,audiences,created_at,expires_at)VALUES('old-key',11,'old','prefix','secret-key-canary','[]','[]',100,9999999999)").execute(&mut src).await.unwrap();
    src.close().await.unwrap();
    let target = db::connect(&destination).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,display_name,password_hash,role,created_at)VALUES(2,'destination','Destination',?,'admin','2026-11-28T00:00:00Z')").bind(hash).execute(&target).await.unwrap();
    sqlx::query("INSERT INTO keywords(id,theme,expression,owner_id,visibility,created_at,updated_at)VALUES(44,'keep','茶',2,'private','2026-11-28T00:00:00Z','2026-11-28T00:00:00Z')").execute(&target).await.unwrap();
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&target)
        .await
        .unwrap();
    target.close().await;
    Fixture {
        _dir: dir,
        source,
        destination,
    }
}

#[tokio::test]
async fn dry_run_is_read_only_explicit_and_never_serializes_credentials() {
    let f = fixture().await;
    let source = fs::read(&f.source).unwrap();
    let target = fs::read(&f.destination).unwrap();
    let report = migration::inspect(&f.source, &f.destination).await.unwrap();
    assert!(report.conflicts.is_empty(), "{:?}", report.conflicts);
    assert_eq!(report.users.len(), 6);
    assert_eq!(
        (report.users[0].source_id, report.users[0].target_id),
        (11, 3)
    );
    assert_eq!(report.excluded["sessions"], 1);
    assert_eq!(report.excluded["access_credentials"], 1);
    let json = serde_json::to_string(&report).unwrap();
    for secret in [
        "$argon2id$",
        "secret-session-canary",
        "secret-key-canary",
        "migration-password-123",
    ] {
        assert!(!json.contains(secret));
    }
    assert_eq!(fs::read(&f.source).unwrap(), source);
    assert_eq!(fs::read(&f.destination).unwrap(), target);
}

#[tokio::test]
async fn apply_preserves_restrictions_ids_backup_and_existing_vault_data() {
    let f = fixture().await;
    let source = fs::read(&f.source).unwrap();
    let before = fs::read(&f.destination).unwrap();
    let report = migration::inspect(&f.source, &f.destination).await.unwrap();
    let backup = f._dir.path().join("backup.db");
    let receipt = migration::apply(&report, &backup).await.unwrap();
    assert_eq!(receipt.imported_users, 6);
    assert_eq!(fs::read(&backup).unwrap(), before);
    assert_eq!(fs::read(&f.source).unwrap(), source);
    let mut target = open(&f.destination).await;
    let preserved: (i64, String, String) =
        sqlx::query_as("SELECT owner_id,visibility,expression FROM keywords WHERE id=44")
            .fetch_one(&mut target)
            .await
            .unwrap();
    assert_eq!(preserved, (2, "private".into(), "茶".into()));
    for (username, status, disabled, enabled) in [
        ("pending", "pending", 0, 1),
        ("rejected", "rejected", 0, 1),
        ("disabled", "active", 1, 1),
        ("provider", "active", 0, 0),
    ] {
        let actual: (String, i64, i64) = sqlx::query_as(
            "SELECT account_status,disabled,password_login_enabled FROM users WHERE username=?",
        )
        .bind(username)
        .fetch_one(&mut target)
        .await
        .unwrap();
        assert_eq!(actual, (status.into(), disabled, enabled));
    }
    let (id, hash): (i64, String) =
        sqlx::query_as("SELECT id,password_hash FROM users WHERE username='admin'")
            .fetch_one(&mut target)
            .await
            .unwrap();
    assert_eq!(id, 3);
    assert!(auth::verify_password("migration-password-123".into(), hash).await);
    let reviewer: i64 =
        sqlx::query_scalar("SELECT reviewed_by FROM access_registrations WHERE status='rejected'")
            .fetch_one(&mut target)
            .await
            .unwrap();
    assert_eq!(reviewer, 3);
    let provider: i64 = sqlx::query_scalar(
        "SELECT user_id FROM access_identities WHERE subject='source-google-subject'",
    )
    .fetch_one(&mut target)
    .await
    .unwrap();
    assert_eq!(provider, 8);
    for table in [
        "sessions",
        "access_credentials",
        "access_grants",
        "access_tokens",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&mut target)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
    target.close().await.unwrap();
    assert!(migration::apply(&report, &f._dir.path().join("second.db"))
        .await
        .is_err());
}

#[tokio::test]
async fn username_provider_hash_and_state_conflicts_block_the_whole_import() {
    let f = fixture().await;
    let mut target = open(&f.destination).await;
    sqlx::query("UPDATE users SET username='AdMiN' WHERE id=2")
        .execute(&mut target)
        .await
        .unwrap();
    sqlx::query("INSERT INTO access_identities(id,user_id,provider,subject,linked_at)VALUES('existing',2,'google','source-google-subject',1)").execute(&mut target).await.unwrap();
    target.close().await.unwrap();
    let mut source = open(&f.source).await;
    sqlx::query("UPDATE users SET password_hash='unsupported-hash' WHERE id=12")
        .execute(&mut source)
        .await
        .unwrap();
    source.close().await.unwrap();
    let report = migration::inspect(&f.source, &f.destination).await.unwrap();
    assert!(report.conflicts.len() >= 3, "{:?}", report.conflicts);
    let before = fs::read(&f.destination).unwrap();
    assert!(migration::apply(&report, &f._dir.path().join("backup.db"))
        .await
        .is_err());
    assert_eq!(fs::read(&f.destination).unwrap(), before);
}

#[tokio::test]
async fn tampered_reports_drift_and_existing_backup_are_rejected_before_writes() {
    let f = fixture().await;
    let report = migration::inspect(&f.source, &f.destination).await.unwrap();
    let mut tampered = report.clone();
    tampered.users[0].target_id = 999;
    let backup = f._dir.path().join("backup.db");
    assert!(migration::apply(&tampered, &backup).await.is_err());
    assert!(!backup.exists());
    fs::write(&backup, b"do not overwrite").unwrap();
    assert!(migration::apply(&report, &backup).await.is_err());
    assert_eq!(fs::read(&backup).unwrap(), b"do not overwrite");
    let mut target = open(&f.destination).await;
    sqlx::query("UPDATE keywords SET theme='changed' WHERE id=44")
        .execute(&mut target)
        .await
        .unwrap();
    target.close().await.unwrap();
    assert!(migration::apply(&report, &f._dir.path().join("new.db"))
        .await
        .is_err());
}

#[tokio::test]
async fn insertion_failure_rolls_back_all_accounts_and_keeps_verified_backup() {
    let f = fixture().await;
    let mut target = open(&f.destination).await;
    sqlx::query("CREATE TRIGGER reject_member BEFORE INSERT ON users WHEN NEW.username='member' BEGIN SELECT RAISE(ABORT,'fixture failure'); END").execute(&mut target).await.unwrap();
    target.close().await.unwrap();
    let report = migration::inspect(&f.source, &f.destination).await.unwrap();
    let before = fs::read(&f.destination).unwrap();
    let backup = f._dir.path().join("backup.db");
    assert!(migration::apply(&report, &backup).await.is_err());
    assert_eq!(fs::read(&backup).unwrap(), before);
    let mut target = open(&f.destination).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&mut target)
        .await
        .unwrap();
    assert_eq!(count, 1);
    target.close().await.unwrap();
}

#[tokio::test]
async fn unsupported_schema_and_uncheckpointed_wal_are_refused() {
    let f = fixture().await;
    let mut source = open(&f.source).await;
    sqlx::query("UPDATE _sqlx_migrations SET checksum=X'00' WHERE version=1")
        .execute(&mut source)
        .await
        .unwrap();
    source.close().await.unwrap();
    assert!(migration::inspect(&f.source, &f.destination).await.is_err());
    let f = fixture().await;
    fs::write(format!("{}-wal", f.source.display()), b"outstanding WAL").unwrap();
    assert!(migration::inspect(&f.source, &f.destination).await.is_err());
}

#[tokio::test]
async fn cli_defaults_to_dry_run_without_initializing_the_configured_application_db() {
    let f = fixture().await;
    let source = fs::read(&f.source).unwrap();
    let destination = fs::read(&f.destination).unwrap();
    let report = f._dir.path().join("report.json");
    let unrelated = f._dir.path().join("must-not-exist.db");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_keyword-vault"))
        .args(["import-compiler", "--source"])
        .arg(&f.source)
        .arg("--destination")
        .arg(&f.destination)
        .arg("--report")
        .arg(&report)
        .env("KV_DATABASE_PATH", &unrelated)
        .env(
            "ACCESS_SETUP_TOKEN",
            "invalid-but-unneeded-for-offline-import",
        )
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(report.is_file());
    assert!(!unrelated.exists());
    assert_eq!(fs::read(&f.source).unwrap(), source);
    assert_eq!(fs::read(&f.destination).unwrap(), destination);
}

#[tokio::test]
async fn inconsistent_registration_and_missing_reviewer_are_explicit_conflicts() {
    let f = fixture().await;
    let mut source = open(&f.source).await;
    sqlx::query("UPDATE access_registrations SET status='active',reviewed_by=999 WHERE id=1")
        .execute(&mut source)
        .await
        .unwrap();
    source.close().await.unwrap();
    let report = migration::inspect(&f.source, &f.destination).await.unwrap();
    assert!(report.conflicts.iter().any(|c| c.contains("inconsistent")));
    assert!(report
        .conflicts
        .iter()
        .any(|c| c.contains("missing reviewer")));
    assert!(migration::apply(&report, &f._dir.path().join("backup.db"))
        .await
        .is_err());
}

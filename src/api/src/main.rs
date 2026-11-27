use anyhow::{bail, Result};
use keyword_vault::{app, auth, build_state, demo, Config};
use std::io::Read;
use tokio::net::TcpListener;
use tracing_subscriber::{prelude::*, EnvFilter};

const USAGE: &str = "用法：
  keyword-vault                          啟動伺服器
  keyword-vault import-compiler --source <舊資料庫> --destination <Vault資料庫> --report <新報告>
  keyword-vault import-compiler --apply <報告> --backup <新備份>
  keyword-vault bootstrap <帳號>         從 stdin 讀取密碼，建立第一位管理員
  keyword-vault reset-password <帳號>    重設某個帳號的密碼，新密碼會印在畫面上
  keyword-vault demo                     在空的資料庫裡建立範例帳號和關鍵字，密碼會印在畫面上
                                         （要自己指定密碼的話設 KV_DEMO_PASSWORD）

設定用環境變數，見 README。";

#[tokio::main]
async fn main() -> Result<()> {
    // SDK debug events may include tool arguments/results. This filter cannot
    // be overridden by a more-specific RUST_LOG directive.
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .with(
            tracing_subscriber::fmt::layer().with_filter(tracing_subscriber::filter::filter_fn(
                |metadata| !metadata.target().starts_with("rmcp"),
            )),
        )
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => serve(Config::from_env()?).await,
        Some("import-compiler") => import_compiler(&args[1..]).await,
        Some("bootstrap") if args.len() == 2 => bootstrap(Config::from_env()?, &args[1]).await,
        Some("reset-password") => match args.get(1) {
            Some(username) => reset_password(Config::from_env()?, username).await,
            None => bail!("{USAGE}"),
        },
        Some("demo") => seed_demo(Config::from_env()?).await,
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => bail!("不認得的指令：{other}\n\n{USAGE}"),
    }
}

async fn serve(config: Config) -> Result<()> {
    tracing::info!(
        "Keyword Vault v{}｜資料庫 {}",
        env!("CARGO_PKG_VERSION"),
        config.database_path.display()
    );
    let bind = config.bind.clone();
    let state = build_state(config).await?;
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    if users == 0 {
        tracing::info!("還沒有任何帳號：打開網頁建立第一個管理員");
    }
    let listener = TcpListener::bind(&bind).await?;
    tracing::info!("網頁介面：http://{}", listener.local_addr()?);
    axum::serve(
        listener,
        app(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    tracing::info!("已停止");
    Ok(())
}

async fn bootstrap(config: Config, username: &str) -> Result<()> {
    let mut password = String::new();
    std::io::stdin().take(4097).read_to_string(&mut password)?;
    if password.len() > 4096 {
        bail!("Password input too long");
    }
    let password = password.trim_end_matches(['\r', '\n']).to_owned();
    let db = keyword_vault::db::connect(&config.database_path).await?;
    let user = keyword_vault::setup::create_admin(&db, username, None, password)
        .await
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    db.close().await;
    println!("Administrator {} created.", user.username);
    Ok(())
}

async fn reset_password(config: Config, username: &str) -> Result<()> {
    let state = build_state(config).await?;
    let id: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(&state.db)
        .await?;
    let Some(id) = id else {
        bail!("找不到帳號：{username}");
    };
    let password = auth::random_hex(10);
    auth::set_password(&state.db, id, &password)
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    auth::delete_sessions_of(&state.db, id, None).await?;
    println!("{username} 的新密碼：{password}");
    println!("請登入後到「設定」改成自己的密碼。");
    Ok(())
}

async fn seed_demo(config: Config) -> Result<()> {
    let state = build_state(config).await?;
    let password = match keyword_vault::config::var("KV_DEMO_PASSWORD") {
        Some(p) => p,
        None => auth::random_hex(8),
    };
    let summary = demo::seed(&state, &password).await?;
    println!(
        "建立了 {} 個帳號、{} 個關鍵字。",
        summary.users, summary.keywords
    );
    println!(
        "管理員帳號：demo　其他帳號：{}",
        demo::MEMBER_USERNAMES.join("、")
    );
    println!("密碼（所有範例帳號都一樣）：{password}");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn import_compiler(args: &[String]) -> Result<()> {
    use keyword_vault::migration::{self, MigrationReport};
    use std::{collections::BTreeMap, path::Path};
    let mut options = BTreeMap::new();
    for pair in args.chunks(2) {
        if pair.len() != 2
            || !matches!(
                pair[0].as_str(),
                "--source" | "--destination" | "--report" | "--apply" | "--backup"
            )
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            bail!("Invalid import arguments.\n{USAGE}");
        }
    }
    if options.len() == 3
        && options.contains_key("--source")
        && options.contains_key("--destination")
        && options.contains_key("--report")
    {
        let report = migration::inspect(
            Path::new(options["--source"]),
            Path::new(options["--destination"]),
        )
        .await?;
        migration::write_report(Path::new(options["--report"]), &report)?;
        println!(
            "Dry-run report written: {} users, {} identities, {} conflicts. No database changes.",
            report.users.len(),
            report.identities.len(),
            report.conflicts.len()
        );
        if !report.conflicts.is_empty() {
            bail!("Import blocked by conflicts; inspect the report and resolve them explicitly.");
        }
        Ok(())
    } else if options.len() == 2
        && options.contains_key("--apply")
        && options.contains_key("--backup")
    {
        let mut input = Vec::new();
        std::fs::File::open(options["--apply"])?
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut input)?;
        if input.len() > 64 * 1024 * 1024 {
            bail!("Migration report exceeds 64 MiB");
        }
        let report: MigrationReport = serde_json::from_slice(&input)?;
        let receipt = migration::apply(&report, Path::new(options["--backup"])).await?;
        println!("{}", serde_json::to_string_pretty(&receipt)?);
        Ok(())
    } else {
        bail!(
            "Use a dry-run source/destination/report, or apply/report with a new backup.\n{USAGE}"
        );
    }
}

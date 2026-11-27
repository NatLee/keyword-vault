use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
    Router,
};
use keyword_vault::{app, build_state, Config};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn fixture() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config::from_env().unwrap();
    config.database_path = dir.path().join("vault.db");
    config.static_dir = dir.path().join("web");
    let state = build_state(config).await.unwrap();
    (dir, app(state))
}
async fn call(router: &Router, peer: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(if body.is_some() { "POST" } else { "GET" })
        .uri(path)
        .extension(ConnectInfo(peer.parse::<std::net::SocketAddr>().unwrap()))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "127.0.0.1")
        .body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn remote_setup_and_forged_forwarding_cannot_claim_the_first_account() {
    let (_dir, router) = fixture().await;
    let (status, _) = call(
        &router,
        "192.0.2.1:1234",
        "/api/auth/setup",
        Some(json!({"username":"admin","password":"test-password-123"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, state) = call(&router, "192.0.2.1:1234", "/api/auth/state", None).await;
    assert_eq!(state["setup_required"], true);
    assert_eq!(state["setup_token_required"], true);
    assert_eq!(state["setup_token_configured"], false);
}
#[tokio::test]
async fn real_loopback_may_bootstrap_once_and_then_remote_setup_is_closed() {
    let (_dir, router) = fixture().await;
    let body = json!({"username":"admin","password":"test-password-123"});
    assert_eq!(
        call(
            &router,
            "127.0.0.1:1234",
            "/api/auth/setup",
            Some(body.clone())
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(&router, "192.0.2.1:1234", "/api/auth/setup", Some(body))
            .await
            .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn configured_deployment_token_is_required_and_never_returned() {
    let dir = tempfile::tempdir().unwrap();
    let secret = "a-valid-deployment-token-at-least-32-chars";
    let mut config = Config::from_env().unwrap();
    config.database_path = dir.path().join("vault.db");
    config.setup_token = Some(keyword_vault::setup::SetupToken::new(secret.into()).unwrap());
    assert!(!format!("{config:?}").contains(secret));
    let router = app(build_state(config).await.unwrap());
    let (_, state) = call(&router, "192.0.2.1:1234", "/api/auth/state", None).await;
    assert_eq!(state["setup_token_configured"], true);
    assert!(!state.to_string().contains(secret));
    for token in ["", "wrong-token"] {
        assert_eq!(
            call(
                &router,
                "192.0.2.1:1234",
                "/api/auth/setup",
                Some(
                    json!({"username":"admin","password":"test-password-123","setup_token":token})
                )
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        call(
            &router,
            "192.0.2.1:1234",
            "/api/auth/setup",
            Some(json!({"username":"admin","password":"test-password-123","setup_token":secret}))
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[test]
fn malformed_deployment_tokens_are_rejected() {
    for token in [
        "".to_owned(),
        " ".repeat(32),
        "a".repeat(31),
        "a".repeat(513),
    ] {
        assert!(keyword_vault::setup::SetupToken::new(token).is_err());
    }
}

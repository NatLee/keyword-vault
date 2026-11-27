//! Access integration uses the same current owner/public/group predicates as cookies.
use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use keyword_vault::{app, auth, build_state, AppState, Config};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    if let Some(token) = bearer {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if body.is_some() {
        req = req.header(header::CONTENT_TYPE, "application/json");
    }
    let response = router
        .clone()
        .oneshot(
            req.body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
async fn fixture() -> (TempDir, AppState, Router) {
    let dir = TempDir::new().unwrap();
    let state = build_state(Config {
        bind: "127.0.0.1:0".into(),
        database_path: dir.path().join("test.db"),
        static_dir: dir.path().join("web"),
        public_origin: None,
        cookie_secure: false,
        default_slop: 20,
        segmentation_dict: None,
        setup_token: None,
    })
    .await
    .unwrap();
    let router = app(state.clone());
    (dir, state, router)
}
async fn user(state: &AppState, name: &str, role: &str) -> (i64, String) {
    let id = auth::create_user(&state.db, name, name, "test-password-123", role)
        .await
        .unwrap();
    let token = auth::create_session(&state.db, id).await.unwrap();
    (id, format!("{}={token}", auth::COOKIE_NAME))
}
async fn key(router: &Router, cookie: &str, scopes: Value, audiences: Value) -> String {
    let (status, value) = call(
        router,
        Method::POST,
        "/api/access/credentials",
        Some(cookie),
        None,
        Some(
            json!({"name":"integration","scopes":scopes,"audiences":audiences,"expires_in_days":1}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    value["token"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn keys_keep_current_visibility_and_audit_once_and_revoke_immediately() {
    let (_dir, state, router) = fixture().await;
    let (owner, owner_cookie) = user(&state, "owner", "admin").await;
    let (member, member_cookie) = user(&state, "member", "member").await;
    let mut ids = Vec::new();
    for visibility in ["public", "private", "group"] {
        let(status,value)=call(&router,Method::POST,"/api/keywords",Some(&owner_cookie),None,Some(json!({"theme":visibility,"expression":"tea|coffee","visibility":visibility,"tags":[]}))).await;
        assert_eq!(status, StatusCode::OK);
        ids.push(value["id"].as_i64().unwrap());
    }
    let group = sqlx::query(
        "INSERT INTO user_groups(name,created_at)VALUES('test','2026-11-27T00:00:00Z')",
    )
    .execute(&state.db)
    .await
    .unwrap()
    .last_insert_rowid();
    for id in [owner, member] {
        sqlx::query("INSERT INTO group_members(group_id,user_id)VALUES(?,?)")
            .bind(group)
            .bind(id)
            .execute(&state.db)
            .await
            .unwrap();
    }
    let (_outsider, admin_cookie) = user(&state, "outsideadmin", "admin").await;
    let admin_token = key(
        &router,
        &admin_cookie,
        json!(["keywords:read"]),
        json!(["api"]),
    )
    .await;
    assert_eq!(
        call(
            &router,
            Method::POST,
            "/api/v1/get_keyword",
            None,
            Some(&admin_token),
            Some(json!({"id":ids[1]}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND,
        "admin role does not bypass Vault private visibility"
    );
    assert_eq!(
        call(
            &router,
            Method::POST,
            "/api/v1/get_keyword",
            Some(&owner_cookie),
            None,
            Some(json!({"id":ids[0]}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED,
        "cookies do not authorize bearer domain endpoints"
    );
    let token = key(
        &router,
        &member_cookie,
        json!(["keywords:read", "expressions:read"]),
        json!(["api", "mcp"]),
    )
    .await;
    for (id, expected) in [
        (ids[0], StatusCode::OK),
        (ids[1], StatusCode::NOT_FOUND),
        (ids[2], StatusCode::OK),
    ] {
        assert_eq!(
            call(
                &router,
                Method::POST,
                "/api/v1/get_keyword",
                None,
                Some(&token),
                Some(json!({"id":id}))
            )
            .await
            .0,
            expected
        );
    }
    sqlx::query("DELETE FROM group_members WHERE user_id=?")
        .bind(member)
        .execute(&state.db)
        .await
        .unwrap();
    assert_eq!(
        call(
            &router,
            Method::POST,
            "/api/v1/get_keyword",
            None,
            Some(&token),
            Some(json!({"id":ids[2]}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (status, inspection) = call(
        &router,
        Method::POST,
        "/api/v1/inspect_expression",
        None,
        Some(&token),
        Some(json!({"expression":"tea|coffee"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(inspection["query_string"], "\"tea\" OR \"coffee\"");
    let (status, usage) = call(
        &router,
        Method::GET,
        "/api/access/usage?transport=api",
        Some(&member_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(usage["totals"]["attempts"], 5);
    assert_eq!(usage["totals"]["successes"], 3);
    assert_eq!(usage["totals"]["failures"], 2);
    let (_, listed) = call(
        &router,
        Method::GET,
        "/api/access/credentials",
        Some(&member_cookie),
        None,
        None,
    )
    .await;
    assert!(listed["items"][0].get("token").is_none());
    assert!(!listed.to_string().contains(&token));
    let id = listed["items"][0]["id"].as_str().unwrap();
    assert_eq!(
        call(
            &router,
            Method::DELETE,
            &format!("/api/access/credentials/{id}"),
            Some(&member_cookie),
            None,
            None
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &router,
            Method::POST,
            "/api/v1/get_keyword",
            None,
            Some(&token),
            Some(json!({"id":ids[0]}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn bearer_scope_audience_disabled_and_forged_header_cannot_escalate() {
    let (_dir, state, router) = fixture().await;
    let (id, cookie) = user(&state, "reader", "member").await;
    let token = key(&router, &cookie, json!(["keywords:read"]), json!(["api"])).await;
    assert_eq!(
        call(
            &router,
            Method::POST,
            "/api/v1/inspect_expression",
            None,
            Some(&token),
            Some(json!({"expression":"a"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &router,
            Method::GET,
            "/api/access/credentials",
            None,
            Some(&token),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/keywords")
                .header("x-trusted-principal", id.to_string())
                .header("x-user-id", id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE users SET disabled=1 WHERE id=?")
        .bind(id)
        .execute(&state.db)
        .await
        .unwrap();
    assert_eq!(
        call(
            &router,
            Method::POST,
            "/api/v1/search_keywords",
            None,
            Some(&token),
            Some(json!({}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn official_stateless_sdk_uses_current_group_visibility_and_counts_each_call_once() {
    use rmcp::{
        model::{CallToolRequestParams, ProtocolVersion},
        transport::{
            streamable_http_client::StreamableHttpClientTransportConfig,
            StreamableHttpClientTransport,
        },
        ClientLifecycleMode, ClientServiceExt,
    };
    use std::future::IntoFuture;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let dir = TempDir::new().unwrap();
    let state = build_state(Config {
        bind: "127.0.0.1:0".into(),
        database_path: dir.path().join("test.db"),
        static_dir: dir.path().join("web"),
        public_origin: Some(origin.clone()),
        cookie_secure: false,
        default_slop: 20,
        segmentation_dict: None,
        setup_token: None,
    })
    .await
    .unwrap();
    let router = app(state.clone());
    let (owner, owner_cookie) = user(&state, "sdkowner", "admin").await;
    let (member, cookie) = user(&state, "sdkreader", "member").await;
    let mut ids = Vec::new();
    for visibility in ["public", "private", "group"] {
        let(status,record)=call(&router,Method::POST,"/api/keywords",Some(&owner_cookie),None,Some(json!({"theme":visibility,"expression":"tea|coffee","visibility":visibility,"tags":[]}))).await;
        assert_eq!(status, StatusCode::OK);
        ids.push(record["id"].as_i64().unwrap());
    }
    let group =
        sqlx::query("INSERT INTO user_groups(name,created_at)VALUES('sdk','2026-11-27T00:00:00Z')")
            .execute(&state.db)
            .await
            .unwrap()
            .last_insert_rowid();
    for id in [owner, member] {
        sqlx::query("INSERT INTO group_members(group_id,user_id)VALUES(?,?)")
            .bind(group)
            .bind(id)
            .execute(&state.db)
            .await
            .unwrap();
    }
    let token = key(&router, &cookie, json!(["keywords:read"]), json!(["mcp"])).await;
    assert_eq!(
        call(
            &router,
            Method::POST,
            "/api/v1/search_keywords",
            None,
            Some(&token),
            Some(json!({}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED,
        "MCP-only keys must not authorize REST operations"
    );
    let server = tokio::spawn(axum::serve(listener, router.clone()).into_future());
    let config = StreamableHttpClientTransportConfig::with_uri(format!("{origin}/mcp"))
        .auth_header(token.clone());
    let client = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        ().serve_with_lifecycle(
            StreamableHttpClientTransport::from_config(config),
            ClientLifecycleMode::Discover {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
            },
        ),
    )
    .await
    .unwrap()
    .expect("official SDK discovery");
    assert_eq!(client.list_tools(None).await.unwrap().tools.len(), 2);
    for (id, denied) in [(ids[0], false), (ids[1], true), (ids[2], false)] {
        let result = client
            .call_tool(
                CallToolRequestParams::new("get_keyword")
                    .with_arguments(json!({"id":id}).as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        let result = serde_json::to_value(result).unwrap();
        assert_eq!(result["isError"].as_bool().unwrap_or(false), denied);
        if !denied {
            assert_eq!(result["structuredContent"]["id"], id);
        }
    }
    sqlx::query("DELETE FROM group_members WHERE user_id=?")
        .bind(member)
        .execute(&state.db)
        .await
        .unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("get_keyword")
                .with_arguments(json!({"id":ids[2]}).as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(serde_json::to_value(result).unwrap()["isError"], true);
    let (_, usage) = call(
        &router,
        Method::GET,
        "/api/access/usage?transport=mcp",
        Some(&cookie),
        None,
        None,
    )
    .await;
    assert_eq!(usage["totals"]["attempts"], 4);
    assert_eq!(usage["totals"]["successes"], 2);
    assert_eq!(usage["totals"]["failures"], 2);
    sqlx::query("UPDATE users SET disabled=1 WHERE id=?")
        .bind(member)
        .execute(&state.db)
        .await
        .unwrap();
    assert!(
        client
            .call_tool(
                CallToolRequestParams::new("get_keyword")
                    .with_arguments(json!({"id":ids[0]}).as_object().unwrap().clone())
            )
            .await
            .is_err(),
        "disabled host accounts cannot keep MCP authority"
    );
    sqlx::query("UPDATE users SET disabled=0 WHERE id=?")
        .bind(member)
        .execute(&state.db)
        .await
        .unwrap();
    let (_, keys) = call(
        &router,
        Method::GET,
        "/api/access/credentials",
        Some(&cookie),
        None,
        None,
    )
    .await;
    let id = keys["items"][0]["id"].as_str().unwrap();
    assert_eq!(
        call(
            &router,
            Method::DELETE,
            &format!("/api/access/credentials/{id}"),
            Some(&cookie),
            None,
            None
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert!(client
        .call_tool(
            CallToolRequestParams::new("get_keyword")
                .with_arguments(json!({"id":ids[0]}).as_object().unwrap().clone())
        )
        .await
        .is_err());
    let _ = client.cancel().await;
    server.abort();
}

#[tokio::test]
async fn compiler_compatibility_preserves_contracts_scope_and_audits_once() {
    let (_dir, state, router) = fixture().await;
    let (_, cookie) = user(&state, "compilerowner", "admin").await;
    let legacy = key(&router, &cookie, json!(["queries:read"]), json!(["api"])).await;
    let current = key(
        &router,
        &cookie,
        json!(["expressions:read"]),
        json!(["api"]),
    )
    .await;
    let (status, result) = call(
        &router,
        Method::POST,
        "/api/v1/compile_query",
        None,
        Some(&legacy),
        Some(json!({"expression":"a|b"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result, json!({"query_string":"\"a\" OR \"b\""}));
    let (_, result) = call(
        &router,
        Method::POST,
        "/api/v1/extract_terms",
        None,
        Some(&legacy),
        Some(json!({"expression":"茶!(廣告|業配)"})),
    )
    .await;
    assert_eq!(result, json!({"terms":["茶","廣告","業配"]}));
    let (status, result) = call(
        &router,
        Method::POST,
        "/api/v1/inspect_query",
        None,
        Some(&legacy),
        Some(json!({"expression":"a&&b"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!result.to_string().contains("不可"));
    let (status, result) = call(
        &router,
        Method::POST,
        "/api/v1/inspect_expression",
        None,
        Some(&current),
        Some(json!({"expression":"a&&b"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["valid"], false);
    assert!(result["error"]["message"]
        .as_str()
        .unwrap()
        .contains("不可"));
    let (status, _) = call(
        &router,
        Method::POST,
        "/api/v1/compile_query",
        None,
        Some(&current),
        Some(json!({"expression":"tea"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let attempts: i64 = sqlx::query_scalar("SELECT SUM(attempts) FROM access_daily")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(
        attempts, 5,
        "each authenticated operation attempt is counted once"
    );
    let (status, _) = call(
        &router,
        Method::POST,
        "/api/compiler/inspect",
        Some(&cookie),
        None,
        Some(json!({"expression":"茶".repeat(40_001)})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &router,
        Method::POST,
        "/api/compiler/inspect",
        Some(&cookie),
        None,
        Some(json!({"expression":"tea","unknown":true})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = call(
        &router,
        Method::POST,
        "/api/compiler/inspect",
        None,
        None,
        Some(json!({"expression":"tea"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, result) = call(
        &router,
        Method::POST,
        "/api/v1/inspect_workbench",
        None,
        Some(&current),
        Some(json!({"expression":"tea|coffee","filter_expression":"shop|cafe"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        result["inspection"]["query_string"],
        r#"("shop" OR "cafe") AND ("tea" OR "coffee")"#
    );
}

#[tokio::test]
async fn chinese_analysis_is_authenticated_bounded_and_scope_limited() {
    let (_dir, state, router) = fixture().await;
    let (_, cookie) = user(&state, "analyzer", "admin").await;
    let (status, _) = call(
        &router,
        Method::POST,
        "/api/tools/analyze",
        None,
        None,
        Some(json!({"expression":"臺積電"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let token = key(
        &router,
        &cookie,
        json!(["expressions:read"]),
        json!(["api"]),
    )
    .await;
    let (status, result) = call(
        &router,
        Method::POST,
        "/api/v1/analyze_expression",
        None,
        Some(&token),
        Some(json!({"expression":"臺積電!廣告"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["leaves"][0]["tokens"][0]["text"], "臺積電");
    let (status, _) = call(
        &router,
        Method::POST,
        "/api/v1/analyze_expression",
        None,
        Some(&token),
        Some(json!({"expression":"tea","dictionary_path":"/etc/passwd"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &router,
        Method::POST,
        "/api/tools/analyze",
        Some(&cookie),
        None,
        Some(json!({"expression":"title:tea*","precise":false})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn official_mcp_sdk_exercises_consolidated_compiler_and_analysis_operations() {
    use rmcp::{
        model::{CallToolRequestParams, ProtocolVersion},
        transport::{
            streamable_http_client::StreamableHttpClientTransportConfig,
            StreamableHttpClientTransport,
        },
        ClientLifecycleMode, ClientServiceExt,
    };
    use std::future::IntoFuture;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let dir = TempDir::new().unwrap();
    let state = build_state(Config {
        bind: "127.0.0.1:0".into(),
        database_path: dir.path().join("test.db"),
        static_dir: dir.path().join("web"),
        public_origin: Some(origin.clone()),
        cookie_secure: false,
        default_slop: 20,
        segmentation_dict: None,
        setup_token: None,
    })
    .await
    .unwrap();
    let router = app(state.clone());
    let (_, cookie) = user(&state, "consolidated", "admin").await;
    let token = key(
        &router,
        &cookie,
        json!(["queries:read", "expressions:read"]),
        json!(["mcp"]),
    )
    .await;
    let server = tokio::spawn(axum::serve(listener, router).into_future());
    let client = ()
        .serve_with_lifecycle(
            StreamableHttpClientTransport::from_config(
                StreamableHttpClientTransportConfig::with_uri(format!("{origin}/mcp"))
                    .auth_header(token),
            ),
            ClientLifecycleMode::Discover {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
            },
        )
        .await
        .unwrap();
    for (operation, args, field, expected) in [
        (
            "compile_query",
            json!({"expression":"a|b"}),
            "query_string",
            json!("\"a\" OR \"b\""),
        ),
        (
            "extract_terms",
            json!({"expression":"茶!廣告"}),
            "terms",
            json!(["茶", "廣告"]),
        ),
        (
            "inspect_query",
            json!({"expression":"(a b ~)","default_slop":3}),
            "query_string",
            json!("(\"a b\"~3)"),
        ),
        (
            "inspect_workbench",
            json!({"expression":"tea","filter_expression":"shop|cafe"}),
            "valid",
            json!(true),
        ),
        (
            "analyze_expression",
            json!({"expression":"珍珠奶茶","mode":"search"}),
            "engine",
            json!("jieba-rs"),
        ),
    ] {
        let result = client
            .call_tool(
                CallToolRequestParams::new(operation)
                    .with_arguments(args.as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        let value = serde_json::to_value(result).unwrap();
        assert_ne!(value["isError"], true, "{operation}: {value}");
        assert_eq!(value["structuredContent"][field], expected);
    }
    let attempts: i64 =
        sqlx::query_scalar("SELECT SUM(attempts) FROM access_daily WHERE transport='mcp'")
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(attempts, 5);
    for operation in ["compile_query", "inspect_query", "extract_terms"] {
        let result = client
            .call_tool(
                CallToolRequestParams::new(operation)
                    .with_arguments(json!({"expression":"a&&b"}).as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        let value = serde_json::to_value(result).unwrap();
        assert_eq!(value["isError"], true, "{operation}");
        assert_eq!(
            value["structuredContent"]["code"], "compile_error",
            "{operation}: {value}"
        );
        assert_eq!(
            value["structuredContent"]["error"],
            es_keyword_query::Query::parse("a&&b")
                .unwrap_err()
                .to_string()
        );
    }
    client.cancel().await.unwrap();
    server.abort();
}

#[tokio::test]
async fn legacy_compiler_http_parser_errors_keep_machine_code_and_message() {
    let (_dir, state, router) = fixture().await;
    let (_, cookie) = user(&state, "legacyerrors", "admin").await;
    let token = key(&router, &cookie, json!(["queries:read"]), json!(["api"])).await;
    let expected = es_keyword_query::Query::parse("a&&b")
        .unwrap_err()
        .to_string();
    for operation in ["compile_query", "inspect_query", "extract_terms"] {
        let (status, result) = call(
            &router,
            Method::POST,
            &format!("/api/v1/{operation}"),
            None,
            Some(&token),
            Some(json!({"expression":"a&&b"})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            result,
            json!({"code":"compile_error", "error":expected}),
            "{operation}"
        );
    }
}

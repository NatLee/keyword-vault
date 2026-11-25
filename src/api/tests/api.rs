//! 整合測試：每個測試用一個全新的暫存資料庫，直接對 router 送請求。

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use keyword_vault::{app, build_state, demo, AppState, Config};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

const PASSWORD: &str = "test-password-123";

struct TestApp {
    router: Router,
    state: AppState,
    _dir: TempDir,
}

/// 某個人登入之後的 cookie。
#[derive(Clone)]
struct Session(String);

impl TestApp {
    async fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let config = Config {
            bind: "127.0.0.1:0".into(),
            database_path: dir.path().join("test.db"),
            static_dir: dir.path().join("web"),
            public_origin: None,
            cookie_secure: false,
            default_slop: 20,
        };
        let state = build_state(config).await.unwrap();
        Self {
            router: app(state.clone()),
            state,
            _dir: dir,
        }
    }

    async fn send(&self, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let resp = self.router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let headers = resp.headers().clone();
        let bytes = resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        (status, headers, bytes)
    }

    async fn call(
        &self,
        session: Option<&Session>,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(Session(cookie)) = session {
            req = req.header(header::COOKIE, cookie);
        }
        let req = match body {
            Some(b) => req
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string()))
                .unwrap(),
            None => req.body(Body::empty()).unwrap(),
        };
        let (status, _, bytes) = self.send(req).await;
        let value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
        (status, value)
    }

    /// 送出登入或設定請求，回傳 cookie。
    async fn sign_in(&self, path: &str, body: Value) -> Session {
        let req = Request::builder()
            .method(Method::POST)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let (status, headers, bytes) = self.send(req).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&bytes)
        );
        let cookie = headers[header::SET_COOKIE].to_str().unwrap();
        assert!(cookie.contains("HttpOnly"));
        Session(cookie.split(';').next().unwrap().to_owned())
    }

    /// 建立管理員 `admin` 並登入。
    async fn admin(&self) -> Session {
        self.sign_in(
            "/api/auth/setup",
            json!({ "username": "admin", "display_name": "管理員", "password": PASSWORD }),
        )
        .await
    }

    /// 由管理員建立一個成員並登入，回傳（編號、session）。
    async fn member(&self, admin: &Session, username: &str) -> (i64, Session) {
        let (status, user) = self
            .call(
                Some(admin),
                Method::POST,
                "/api/users",
                Some(json!({ "username": username, "password": PASSWORD })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{user}");
        let session = self
            .sign_in(
                "/api/auth/login",
                json!({ "username": username, "password": PASSWORD }),
            )
            .await;
        (user["id"].as_i64().unwrap(), session)
    }

    async fn create(&self, session: &Session, body: Value) -> Value {
        let (status, keyword) = self
            .call(Some(session), Method::POST, "/api/keywords", Some(body))
            .await;
        assert_eq!(status, StatusCode::OK, "{keyword}");
        keyword
    }

    async fn get(&self, session: &Session, path: &str) -> Value {
        let (status, value) = self.call(Some(session), Method::GET, path, None).await;
        assert_eq!(status, StatusCode::OK, "{path}: {value}");
        value
    }
}

fn ids(list: &Value) -> Vec<i64> {
    let mut ids: Vec<i64> = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["id"].as_i64().unwrap())
        .collect();
    ids.sort_unstable();
    ids
}

#[tokio::test]
async fn first_visitor_creates_the_admin_and_there_is_no_default_account() {
    let t = TestApp::new().await;
    let (status, state) = t.call(None, Method::GET, "/api/auth/state", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state, json!({ "setup_required": true, "user": null }));

    // 還沒登入：什麼都讀不到
    let (status, _) = t.call(None, Method::GET, "/api/keywords", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = t
        .call(
            None,
            Method::POST,
            "/api/auth/login",
            Some(json!({ "username": "admin", "password": "admin" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = t
        .call(
            None,
            Method::POST,
            "/api/auth/setup",
            Some(json!({ "username": "admin", "password": "short" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let admin = t.admin().await;
    let state = t.get(&admin, "/api/auth/state").await;
    assert_eq!(state["setup_required"], false);
    assert_eq!(state["user"]["role"], "admin");
    assert_eq!(state["user"]["display_name"], "管理員");

    // 設定只能做一次
    let (status, _) = t
        .call(
            None,
            Method::POST,
            "/api/auth/setup",
            Some(json!({ "username": "other", "password": PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // 登出之後 cookie 失效
    let (status, _) = t
        .call(Some(&admin), Method::POST, "/api/auth/logout", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = t
        .call(Some(&admin), Method::GET, "/api/keywords", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 密碼沒有被存成明文
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users")
        .fetch_one(&t.state.db)
        .await
        .unwrap();
    assert!(hash.starts_with("$argon2") && !hash.contains(PASSWORD));
}

#[tokio::test]
async fn writes_from_another_site_are_rejected() {
    let t = TestApp::new().await;
    let admin = t.admin().await;
    let request = |origin: &'static str| {
        Request::builder()
            .method(Method::POST)
            .uri("/api/keywords")
            .header(header::HOST, "localhost:8686")
            .header(header::ORIGIN, origin)
            .header(header::COOKIE, &admin.0)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "theme": "t", "expression": "a|b" }).to_string(),
            ))
            .unwrap()
    };
    let (status, _, _) = t.send(request("https://evil.example")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = t.send(request("http://localhost:8686")).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn keyword_lifecycle_with_history_and_tools() {
    let t = TestApp::new().await;
    let admin = t.admin().await;

    let (status, body) = t
        .call(
            Some(&admin),
            Method::POST,
            "/api/keywords",
            Some(json!({ "theme": "壞掉的", "expression": "咖啡&&拿鐵" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"],
        "關鍵字語法錯誤：不可有連續的邏輯運算子（第 4 個字元）"
    );

    let kw = t
        .create(
            &admin,
            json!({ "theme": " 咖啡 ", "expression": " 咖啡|拿鐵 ", "tags": ["#飲料", "飲料", " 熱門 "] }),
        )
        .await;
    let id = kw["id"].as_i64().unwrap();
    assert_eq!(kw["theme"], "咖啡");
    assert_eq!(kw["expression"], "咖啡|拿鐵");
    assert_eq!(kw["visibility"], "private");
    assert_eq!(kw["tags"], json!(["熱門", "飲料"]));
    assert_eq!(kw["is_combination"], false);
    assert_eq!(kw["can_edit"], true);

    let detail = t.get(&admin, &format!("/api/keywords/{id}")).await;
    assert_eq!(detail["query_string"], r#""咖啡" OR "拿鐵""#);
    assert_eq!(detail["terms"], json!(["咖啡", "拿鐵"]));

    // 改語法會留下修改前的內容；只改主題不會
    let path = format!("/api/keywords/{id}");
    let (status, body) = t
        .call(
            Some(&admin),
            Method::PATCH,
            &path,
            Some(json!({ "expression": "咖啡|拿鐵|(精品 咖啡 ~5)" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["keyword"]["expression"], "咖啡|拿鐵|(精品 咖啡 ~5)");
    let (status, _) = t
        .call(
            Some(&admin),
            Method::PATCH,
            &path,
            Some(json!({ "theme": "咖啡類", "tags": [] })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let history = t.get(&admin, &format!("{path}/history")).await;
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert_eq!(history[0]["expression"], "咖啡|拿鐵");
    assert_eq!(history[0]["user_name"], "管理員");
    assert_eq!(
        t.get(&admin, "/api/tags").await,
        json!([]),
        "沒人用的標籤會清掉"
    );

    // 搜尋和篩選
    t.create(
        &admin,
        json!({ "theme": "茶", "expression": "紅茶|綠茶", "tags": ["飲料"] }),
    )
    .await;
    let list = t.get(&admin, "/api/keywords?q=%E7%B2%BE%E5%93%81").await;
    assert_eq!(ids(&list), [id]);
    assert_eq!(list["total"], 1);
    let list = t.get(&admin, "/api/keywords?tag=%E9%A3%B2%E6%96%99").await;
    assert_eq!(list["total"], 1);
    assert_eq!(list["items"][0]["theme"], "茶");
    let list = t.get(&admin, "/api/keywords?page_size=1&sort=theme").await;
    assert_eq!(
        (
            list["total"].as_i64(),
            list["items"].as_array().unwrap().len()
        ),
        (Some(2), 1)
    );
    let (status, _) = t
        .call(Some(&admin), Method::GET, "/api/keywords?scope=x", None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 檢查工具
    let (_, ok) = t
        .call(
            Some(&admin),
            Method::POST,
            "/api/tools/check",
            Some(json!({ "expression": "(a|b)!c" })),
        )
        .await;
    assert_eq!(ok["valid"], true);
    assert_eq!(ok["query_string"], r#"("a" OR "b") NOT "c""#);
    let (_, raw) = t
        .call(
            Some(&admin),
            Method::POST,
            "/api/tools/check",
            Some(json!({ "expression": "iphone*|b", "precise": false })),
        )
        .await;
    assert_eq!(raw["query_string"], "iphone* OR b");
    let (status, bad) = t
        .call(
            Some(&admin),
            Method::POST,
            "/api/tools/check",
            Some(json!({ "expression": "(a|b" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bad["valid"], false);
    assert_eq!(bad["error"]["message"], "不對稱的括號");

    let (_, diff) = t
        .call(
            Some(&admin),
            Method::POST,
            "/api/tools/diff",
            Some(json!({ "a": "a&b", "b": "a|b" })),
        )
        .await;
    assert_eq!(diff["same"], false);
    assert_eq!(
        diff["segments"],
        json!([
            { "op": "equal", "text": "a" },
            { "op": "delete", "text": "&" },
            { "op": "insert", "text": "|" },
            { "op": "equal", "text": "b" },
        ])
    );

    let (status, _) = t.call(Some(&admin), Method::DELETE, &path, None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = t.call(Some(&admin), Method::GET, &path, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn combination_keywords_expand_and_follow_their_parts() {
    let t = TestApp::new().await;
    let admin = t.admin().await;
    let a = t
        .create(&admin, json!({ "theme": "A", "expression": "a1|a2" }))
        .await["id"]
        .as_i64()
        .unwrap();
    let b = t
        .create(&admin, json!({ "theme": "B", "expression": "b1" }))
        .await["id"]
        .as_i64()
        .unwrap();
    let c = t
        .create(&admin, json!({ "theme": "C", "expression": "c1|c2" }))
        .await["id"]
        .as_i64()
        .unwrap();

    // 每個被引用的關鍵字都包在自己的括號裡
    let ab = t
        .create(
            &admin,
            json!({ "theme": "A 和 B", "logic": format!("{a} ＆ #{b}") }),
        )
        .await;
    let ab_id = ab["id"].as_i64().unwrap();
    assert_eq!(ab["logic"], format!("#{a}&#{b}"));
    assert_eq!(ab["expression"], "(a1|a2)&(b1)");
    assert_eq!(ab["is_combination"], true);
    // 組合也可以再被組合
    let nested = t
        .create(
            &admin,
            json!({ "theme": "(A 和 B) 排除 C", "logic": format!("#{ab_id}!#{c}") }),
        )
        .await;
    let nested_id = nested["id"].as_i64().unwrap();
    assert_eq!(nested["expression"], "((a1|a2)&(b1))!(c1|c2)");

    let detail = t.get(&admin, &format!("/api/keywords/{ab_id}")).await;
    assert_eq!(detail["query_string"], r#"("a1" OR "a2") AND ("b1")"#);
    assert_eq!(
        detail["refs"],
        json!([{ "id": a, "theme": "A" }, { "id": b, "theme": "B" }])
    );
    assert_eq!(
        detail["dependents"],
        json!([{ "id": nested_id, "theme": "(A 和 B) 排除 C" }])
    );

    for (body, why) in [
        (
            json!({ "theme": "x", "logic": format!("#{a}") }),
            "只有一個",
        ),
        (
            json!({ "theme": "x", "logic": format!("#{a}&#999") }),
            "不存在",
        ),
        (
            json!({ "theme": "x", "logic": format!("#{a}&&#{b}") }),
            "語法",
        ),
        (
            json!({ "theme": "x", "logic": format!("#{a}&#{b}"), "expression": "a" }),
            "兩個都給",
        ),
        (json!({ "theme": "x" }), "都沒給"),
    ] {
        let (status, _) = t
            .call(Some(&admin), Method::POST, "/api/keywords", Some(body))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{why}");
    }

    // 改 A：兩層組合都跟著更新，並留下紀錄
    let (status, body) = t
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/keywords/{a}"),
            Some(json!({ "expression": "a3" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["cascaded"], 2);
    let ab = t.get(&admin, &format!("/api/keywords/{ab_id}")).await;
    assert_eq!(ab["expression"], "(a3)&(b1)");
    let nested = t.get(&admin, &format!("/api/keywords/{nested_id}")).await;
    assert_eq!(nested["expression"], "((a3)&(b1))!(c1|c2)");
    let history = t
        .get(&admin, &format!("/api/keywords/{nested_id}/history"))
        .await;
    assert_eq!(history[0]["expression"], "((a1|a2)&(b1))!(c1|c2)");

    // 不能直接改組合關鍵字的語法；不能繞一圈引用自己
    let (status, _) = t
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/keywords/{ab_id}"),
            Some(json!({ "expression": "x" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    for target in [ab_id, nested_id] {
        let (status, body) = t
            .call(
                Some(&admin),
                Method::PATCH,
                &format!("/api/keywords/{ab_id}"),
                Some(json!({ "logic": format!("#{a}|#{target}") })),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    let (status, _) = t
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/keywords/{a}"),
            Some(json!({ "logic": format!("#{b}|#{c}") })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "一般關鍵字不能變成組合");

    // 改組合方式
    let (status, body) = t
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/keywords/{ab_id}"),
            Some(json!({ "logic": format!("#{a}|#{c}") })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["keyword"]["expression"], "(a3)|(c1|c2)");
    assert_eq!(body["cascaded"], 1);
    let b_detail = t.get(&admin, &format!("/api/keywords/{b}")).await;
    assert_eq!(b_detail["dependents"], json!([]));

    // 有人在用的關鍵字要明確說要一起刪
    let (status, body) = t
        .call(
            Some(&admin),
            Method::DELETE,
            &format!("/api/keywords/{a}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (status, body) = t
        .call(
            Some(&admin),
            Method::DELETE,
            &format!("/api/keywords/{a}?cascade=true"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["deleted"], 3);
    assert_eq!(ids(&t.get(&admin, "/api/keywords").await), [b, c]);
}

#[tokio::test]
async fn sharing_follows_visibility_and_groups() {
    let t = TestApp::new().await;
    let admin = t.admin().await;
    let (amy_id, amy) = t.member(&admin, "amy").await;
    let (ben_id, ben) = t.member(&admin, "ben").await;
    let (_, cat) = t.member(&admin, "cat").await;

    // 成員不能管理帳號和群組
    let (status, _) = t.call(Some(&amy), Method::GET, "/api/users", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = t
        .call(
            Some(&amy),
            Method::POST,
            "/api/groups",
            Some(json!({ "name": "x" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, group) = t
        .call(
            Some(&admin),
            Method::POST,
            "/api/groups",
            Some(json!({ "name": "行銷", "member_ids": [amy_id, ben_id] })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{group}");
    assert_eq!(
        t.get(&amy, "/api/groups").await[0]["members"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(t.get(&cat, "/api/groups").await, json!([]));

    let private = t
        .create(&amy, json!({ "theme": "私人", "expression": "p" }))
        .await["id"]
        .as_i64()
        .unwrap();
    let grouped = t
        .create(
            &amy,
            json!({ "theme": "群組", "expression": "g", "visibility": "group" }),
        )
        .await["id"]
        .as_i64()
        .unwrap();
    let public = t
        .create(
            &amy,
            json!({ "theme": "公開", "expression": "u", "visibility": "public" }),
        )
        .await["id"]
        .as_i64()
        .unwrap();

    assert_eq!(
        ids(&t.get(&amy, "/api/keywords").await),
        [private, grouped, public]
    );
    assert_eq!(ids(&t.get(&ben, "/api/keywords").await), [grouped, public]);
    assert_eq!(ids(&t.get(&cat, "/api/keywords").await), [public]);
    assert_eq!(
        ids(&t.get(&admin, "/api/keywords").await),
        [public],
        "管理員也看不到別人的私人關鍵字"
    );
    assert_eq!(
        ids(&t.get(&ben, "/api/keywords?scope=mine").await),
        Vec::<i64>::new()
    );
    assert_eq!(
        ids(&t.get(&ben, "/api/keywords?scope=shared").await),
        [grouped, public]
    );

    let (status, _) = t
        .call(
            Some(&cat),
            Method::GET,
            &format!("/api/keywords/{grouped}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let kw = t.get(&ben, &format!("/api/keywords/{grouped}")).await;
    assert_eq!(
        (kw["can_edit"].as_bool(), kw["can_delete"].as_bool()),
        (Some(false), Some(false))
    );
    let (status, _) = t
        .call(
            Some(&ben),
            Method::PATCH,
            &format!("/api/keywords/{grouped}"),
            Some(json!({ "theme": "改掉" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = t
        .call(
            Some(&ben),
            Method::DELETE,
            &format!("/api/keywords/{grouped}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 別人可以拿看得到的關鍵字來組合，但組不了看不到的
    let own = t
        .create(&ben, json!({ "theme": "ben 的", "expression": "x" }))
        .await["id"]
        .as_i64()
        .unwrap();
    let combined = t
        .create(
            &ben,
            json!({ "theme": "組合", "logic": format!("#{own}|#{grouped}") }),
        )
        .await;
    assert_eq!(combined["expression"], "(x)|(g)");
    let (status, _) = t
        .call(
            Some(&ben),
            Method::POST,
            "/api/keywords",
            Some(json!({ "theme": "組合", "logic": format!("#{own}|#{private}") })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // 被別人用到的關鍵字，擁有者不能連帶刪掉別人的東西；看不到的組合也不會露出主題
    let (status, _) = t
        .call(
            Some(&amy),
            Method::DELETE,
            &format!("/api/keywords/{grouped}?cascade=true"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let detail = t.get(&amy, &format!("/api/keywords/{grouped}")).await;
    assert_eq!(detail["dependents"][0]["theme"], Value::Null);

    // 收藏和留言
    let (status, _) = t
        .call(
            Some(&ben),
            Method::PUT,
            &format!("/api/keywords/{public}/favorite"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        ids(&t.get(&ben, "/api/keywords?scope=favorites").await),
        [public]
    );
    assert_eq!(
        ids(&t.get(&amy, "/api/keywords?scope=favorites").await),
        Vec::<i64>::new()
    );
    let (status, _) = t
        .call(
            Some(&cat),
            Method::PUT,
            &format!("/api/keywords/{grouped}/favorite"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, comment) = t
        .call(
            Some(&ben),
            Method::POST,
            &format!("/api/keywords/{grouped}/comments"),
            Some(json!({ "body": " 這個不錯 " })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{comment}");
    assert_eq!(comment["body"], "這個不錯");
    let comment_path = format!("/api/comments/{}", comment["id"]);
    let comments = t
        .get(&amy, &format!("/api/keywords/{grouped}/comments"))
        .await;
    assert_eq!(comments[0]["can_edit"], false);
    assert_eq!(
        t.get(&amy, &format!("/api/keywords/{grouped}")).await["comment_count"],
        1
    );
    let (status, _) = t
        .call(
            Some(&amy),
            Method::PATCH,
            &comment_path,
            Some(json!({ "body": "x" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = t
        .call(Some(&cat), Method::DELETE, &comment_path, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "看不到關鍵字就看不到留言");
    let (status, _) = t
        .call(
            Some(&ben),
            Method::PATCH,
            &comment_path,
            Some(json!({ "body": "改過了" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 停用帳號後，原本的登入馬上失效
    let (status, _) = t
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/users/{ben_id}"),
            Some(json!({ "disabled": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = t.call(Some(&ben), Method::GET, "/api/keywords", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = t
        .call(
            Some(&admin),
            Method::PATCH,
            "/api/users/1",
            Some(json!({ "role": "member" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "管理員不能把自己降級");
}

#[tokio::test]
async fn csv_import_is_all_or_nothing_and_export_round_trips() {
    let t = TestApp::new().await;
    let admin = t.admin().await;
    let import = |csv: &'static str| {
        Request::builder()
            .method(Method::POST)
            .uri("/api/keywords/import")
            .header(header::COOKIE, &admin.0)
            .header(header::CONTENT_TYPE, "text/csv")
            .body(Body::from(csv))
            .unwrap()
    };

    let (status, _, body) = t
        .send(import(
            "主題,關鍵字,分享範圍,標籤\n咖啡,咖啡|拿鐵,公開,飲料;熱門\n壞的,咖啡&&拿鐵,私人,\n也壞,a|b,隨便,\n",
        ))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["rows"].as_array().unwrap().len(), 2, "{body}");
    assert_eq!(body["rows"][0]["line"], 3);
    assert_eq!(body["rows"][1]["theme"], "也壞");
    assert_eq!(t.get(&admin, "/api/keywords").await["total"], 0);

    // 舊版平台的欄位名稱也認得；開頭有 BOM 也可以
    let (status, _, body) = t
        .send(import(
            "\u{feff}主題,關鍵字,是否分享（是/否）\n咖啡,咖啡|拿鐵,是\n\"茶, 各種\",\"紅茶|(無糖 綠茶 ~)\",否\n",
        ))
        .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap()["created"],
        2
    );
    let list = t.get(&admin, "/api/keywords?sort=theme").await;
    assert_eq!(list["items"][0]["visibility"], "public");
    assert_eq!(list["items"][1]["theme"], "茶, 各種");
    assert_eq!(list["items"][1]["visibility"], "private");

    let (status, _, _) = t.send(import("名稱,內容\na,b\n")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let req = Request::builder()
        .uri("/api/keywords/export?sort=theme")
        .header(header::COOKIE, &admin.0)
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = t.send(req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .contains("keywords.csv"));
    let text = String::from_utf8(body).unwrap();
    let lines: Vec<&str> = text.trim_start_matches('\u{feff}').lines().collect();
    assert_eq!(lines[0], "主題,關鍵字,分享範圍,標籤,擁有者,query_string");
    assert_eq!(
        lines[1],
        r#"咖啡,咖啡|拿鐵,公開,,管理員,"""咖啡"" OR ""拿鐵""""#
    );
    assert_eq!(lines.len(), 3);
}

#[tokio::test]
async fn demo_data_builds_and_the_overview_counts_it() {
    let t = TestApp::new().await;
    let summary = demo::seed(&t.state, PASSWORD).await.unwrap();
    assert_eq!(summary.users, 4);
    assert!(
        demo::seed(&t.state, PASSWORD).await.is_err(),
        "只能建在空的資料庫"
    );

    let demo = t
        .sign_in(
            "/api/auth/login",
            json!({ "username": "demo", "password": PASSWORD }),
        )
        .await;
    let stats = t.get(&demo, "/api/stats").await;
    let total = stats["total"].as_i64().unwrap();
    assert!(total > 30 && total < summary.keywords as i64, "{stats}");
    assert_eq!(stats["monthly"].as_array().unwrap().len(), 12);
    let in_last_year: i64 = stats["monthly"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["count"].as_i64().unwrap())
        .sum();
    assert_eq!(in_last_year, total, "範例資料都落在最近 12 個月");
    assert_eq!(
        stats["mine"].as_i64().unwrap(),
        stats["mine_private"].as_i64().unwrap() + stats["mine_shared"].as_i64().unwrap()
    );
    assert!(stats["today"]["id"].is_i64());
    assert_eq!(stats["recent"].as_array().unwrap().len(), 6);
    assert_eq!(stats["favorites"], 5);

    let tags = t.get(&demo, "/api/tags").await;
    assert_eq!(tags[0]["name"], "飲料");

    let random = t.get(&demo, "/api/keywords/random").await;
    assert_ne!(random["visibility"], "private");

    // 兩層的組合有正確展開
    let list = t
        .get(&demo, "/api/keywords?kind=combination&page_size=100")
        .await;
    let nested = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["theme"] == "飲料全品類（去雜訊）")
        .unwrap();
    assert!(nested["expression"]
        .as_str()
        .unwrap()
        .starts_with("((珍珠奶茶|"));

    // 成員看不到別人的私人關鍵字
    let kevin = t
        .sign_in(
            "/api/auth/login",
            json!({ "username": "kevin", "password": PASSWORD }),
        )
        .await;
    let list = t.get(&kevin, "/api/keywords?q=%E5%9C%B0%E9%9C%87").await;
    assert_eq!(list["total"], 0);
}

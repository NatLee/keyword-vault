//! 不存檔的小工具：檢查語法、比對兩段語法。

use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    auth::CurrentUser,
    error::{AppError, AppResult},
    state::AppState,
    syntax::{self, MAX_EXPANDED_CHARS},
};

pub async fn inspect(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(input): Json<crate::workbench::InspectInput>,
) -> AppResult<Json<Value>> {
    Ok(Json(crate::workbench::inspect(
        input,
        state.config.default_slop,
    )?))
}

pub async fn analyze(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(input): Json<crate::analysis::AnalysisInput>,
) -> AppResult<Json<crate::analysis::AnalysisResult>> {
    let permit = state
        .analysis_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::TooManyRequests("斷詞工作忙碌中，請稍後重試".into()))?;
    let analyzer = state.analyzer.clone();
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        analyzer.analyze(input)
    })
    .await??;
    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct CheckBody {
    expression: String,
    /// 每個詞要不要加雙引號（完全比對）。預設要。
    precise: Option<bool>,
}

pub async fn check(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(body): Json<CheckBody>,
) -> AppResult<Json<Value>> {
    if body.expression.chars().count() > MAX_EXPANDED_CHARS {
        return Err(AppError::bad("內容太長了"));
    }
    let precise = body.precise.unwrap_or(true);
    Ok(Json(
        match syntax::compile(&body.expression, state.config.default_slop, precise) {
            Ok(compiled) => json!({
                "valid": true,
                "query_string": compiled.query_string,
                "terms": compiled.terms,
                "clauses": compiled.clauses,
            }),
            Err(err) => json!({
                "valid": false,
                "error": { "message": err.message_zh(), "position": err.position },
            }),
        },
    ))
}

#[derive(Deserialize)]
pub struct DiffBody {
    a: String,
    b: String,
}

pub async fn diff(_user: CurrentUser, Json(body): Json<DiffBody>) -> AppResult<Json<Value>> {
    // 逐字比對的成本和兩邊長度的乘積有關
    const MAX_CHARS: usize = 10_000;
    if body.a.chars().count() > MAX_CHARS || body.b.chars().count() > MAX_CHARS {
        return Err(AppError::bad(format!("比對的內容最多 {MAX_CHARS} 個字元")));
    }
    let (a, b) = (body.a, body.b);
    let segments = tokio::task::spawn_blocking(move || {
        let segments = syntax::diff(&a, &b);
        (a == b, segments)
    })
    .await?;
    Ok(Json(json!({ "same": segments.0, "segments": segments.1 })))
}

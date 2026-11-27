//! Explicit legacy compiler contracts; Vault inspection has different errors.
use crate::{
    auth::CurrentUser,
    error::{AppError, AppResult},
    state::AppState,
    syntax,
};
use access_kit::DomainOperation;
use axum::{
    extract::{Path, State},
    Json,
};
use es_keyword_query::{Options, Query};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub expression: String,
    pub precise: Option<bool>,
    pub default_slop: Option<u32>,
}

pub fn operations() -> Vec<DomainOperation> {
    [
        ("compile_query", "Compile an expression using the unchanged Rust library"),
        ("inspect_query", "Inspect written groups, exclusions and terms with the Rust library"),
        ("extract_terms", "Extract normalized terms in the Rust library's written order"),
    ].into_iter().map(|(name, description)| DomainOperation {
        name, description, required_scope: "queries:read", read_only: true,
        input_schema: json!({"type":"object","additionalProperties":false,"required":["expression"],"properties":{
            "expression":{"type":"string","maxLength":40000},"precise":{"type":"boolean"},
            "default_slop":{"type":"integer","minimum":0,"maximum":4294967295_u64}
        }}),
    }).collect()
}

pub fn evaluate(input: Input, operation: &str, default_slop: u32) -> AppResult<Value> {
    if input.expression.chars().count() > 40_000 {
        return Err(AppError::bad(
            "Expression must be at most 40,000 characters",
        ));
    }
    let query = Query::parse_with(
        &input.expression,
        Options {
            precise: input.precise.unwrap_or(true),
            default_slop: input.default_slop.unwrap_or(default_slop),
            ..Options::default()
        },
    )
    .map_err(|error| AppError::bad(error.to_string()))?;
    match operation {
        "compile_query" => Ok(json!({"query_string":query.to_query_string()})),
        "extract_terms" => Ok(json!({"terms":query.terms()})),
        "inspect_query" => Ok(serde_json::to_value(syntax::describe(&query))?),
        _ => Err(AppError::not_found("Unknown compiler operation")),
    }
}

pub async fn inspect(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(input): Json<Input>,
) -> AppResult<Json<Value>> {
    Ok(Json(evaluate(
        input,
        "inspect_query",
        state.config.default_slop,
    )?))
}

pub async fn invoke(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(operation): Path<String>,
    Json(input): Json<Input>,
) -> AppResult<Json<Value>> {
    Ok(Json(evaluate(
        input,
        &operation,
        state.config.default_slop,
    )?))
}

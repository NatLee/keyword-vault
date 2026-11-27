//! Inspection artifacts keep authored sources separate from preview options.
use es_keyword_query::{Options, Query};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    error::{AppError, AppResult},
    syntax,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectInput {
    pub expression: String,
    pub filter_expression: Option<String>,
    pub precise: Option<bool>,
    pub default_slop: Option<u32>,
}

pub fn inspect(input: InspectInput, default_slop: u32) -> AppResult<Value> {
    let filter = input
        .filter_expression
        .as_deref()
        .filter(|s| !s.trim().is_empty());
    if input.expression.chars().count() + filter.map_or(0, |s| s.chars().count())
        > syntax::MAX_EXPANDED_CHARS
    {
        return Err(AppError::bad("來源與篩選條件合計最多 50,000 個字元"));
    }
    let precise = input.precise.unwrap_or(true);
    let slop = input.default_slop.unwrap_or(default_slop);
    let options = Options {
        precise,
        default_slop: slop,
        ..Options::default()
    };
    let parse = |source: &str, value: &str| {
        Query::parse_with(value, options.clone()).map_err(|error| {
        json!({"valid":false,"error":{"source":source,"message":error.message_zh(),"position":error.position}})
    })
    };
    let query = match parse("expression", &input.expression) {
        Ok(query) => query,
        Err(error) => return Ok(error),
    };
    let filter_query = match filter.map(|f| parse("filter", f)).transpose() {
        Ok(query) => query,
        Err(error) => return Ok(error),
    };
    let output = filter_query
        .as_ref()
        .map_or_else(|| query.to_query_string(), |f| query.filtered_by(f));
    Ok(json!({"valid":true,"inspection":{
        "schema_version":1,
        "expression":input.expression,
        "filter_expression":filter,
        "options":{"precise":precise,"default_slop":slop},
        "query":syntax::describe(&query),
        "filter":filter_query.as_ref().map(syntax::describe),
        "query_string":output,
    }}))
}

use es_keyword_query::{ClauseKind, Joiner, Options, Query};
use serde_json::{json, Value};
use unicode_normalization::UnicodeNormalization;

pub fn inspect(input: &str, precise: bool, default_slop: u32) -> Value {
    let normalized: String = input.nfkc().collect();
    match Query::parse_with(
        input,
        Options {
            precise,
            default_slop,
            ..Options::default()
        },
    ) {
        Ok(query) => {
            let clauses: Vec<Value> = query.clauses().into_iter().map(|c| json!({
                "parent": c.parent,
                "kind": match c.kind { ClauseKind::Term => "term", ClauseKind::Proximity => "proximity", ClauseKind::Group => "group" },
                "joiner": c.joiner.map(|j| match j { Joiner::And => "AND", Joiner::Or => "OR" }),
                "negated": c.negated, "text": c.text, "slop": c.slop, "output": c.output
            })).collect();
            json!({"valid": true, "normalized": normalized, "query_string": query.to_query_string(), "terms": query.terms(), "clauses": clauses})
        }
        Err(error) => {
            json!({"valid": false, "normalized": normalized, "error": {"message": error.message(), "position": error.position}})
        }
    }
}

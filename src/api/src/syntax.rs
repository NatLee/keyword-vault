//! 關鍵字語法的檢查、編譯和比對。語法本身由 `es-keyword-query` 解析。

use es_keyword_query::{ClauseKind, Error, Joiner, Options, Query};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};

/// 一般關鍵字的長度上限（字元）。
pub const MAX_EXPRESSION_CHARS: usize = 5_000;
/// 組合關鍵字展開後的長度上限（字元）。
pub const MAX_EXPANDED_CHARS: usize = 50_000;

/// The Rust parser's explicit grouping, serialized without inferred precedence.
#[derive(Debug, Serialize)]
pub struct Clause {
    parent: Option<usize>,
    kind: &'static str,
    joiner: Option<&'static str>,
    negated: bool,
    text: Option<String>,
    slop: Option<u32>,
    output: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Compiled {
    pub query_string: String,
    /// 語法裡出現的每個詞，照出現順序。
    pub terms: Vec<String>,
    pub clauses: Vec<Clause>,
}

pub fn compile(expression: &str, default_slop: u32, precise: bool) -> Result<Compiled, Error> {
    let options = Options {
        precise,
        default_slop,
        ..Options::default()
    };
    let query = Query::parse_with(expression, options)?;
    Ok(Compiled {
        query_string: query.to_query_string(),
        terms: query.terms().into_iter().map(str::to_owned).collect(),
        clauses: query
            .clauses()
            .into_iter()
            .map(|c| Clause {
                parent: c.parent,
                kind: match c.kind {
                    ClauseKind::Term => "term",
                    ClauseKind::Proximity => "proximity",
                    ClauseKind::Group => "group",
                },
                joiner: c.joiner.map(|j| match j {
                    Joiner::And => "AND",
                    Joiner::Or => "OR",
                }),
                negated: c.negated,
                text: c.text,
                slop: c.slop,
                output: c.output,
            })
            .collect(),
    })
}

/// 給使用者看的錯誤訊息，位置從 1 開始算。
pub fn error_text(err: &Error) -> String {
    format!("{}（第 {} 個字元）", err.message_zh(), err.position + 1)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Segment {
    /// `equal`、`insert`（只在 b）或 `delete`（只在 a）
    pub op: &'static str,
    pub text: String,
}

/// 逐字比對兩段語法，相鄰的同類變更合併成一段。
pub fn diff(a: &str, b: &str) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    for change in TextDiff::from_chars(a, b).iter_all_changes() {
        let op = match change.tag() {
            ChangeTag::Equal => "equal",
            ChangeTag::Insert => "insert",
            ChangeTag::Delete => "delete",
        };
        match out.last_mut() {
            Some(last) if last.op == op => last.text.push_str(change.value()),
            _ => out.push(Segment {
                op,
                text: change.value().to_owned(),
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_and_lists_terms() {
        let c = compile("(珍珠奶茶|(珍珠 鮮奶 ~))!廣告", 20, true).unwrap();
        assert_eq!(
            c.query_string,
            r#"("珍珠奶茶" OR ("珍珠 鮮奶"~20)) NOT "廣告""#
        );
        assert_eq!(c.terms, ["珍珠奶茶", "珍珠 鮮奶", "廣告"]);
        let c = compile("(a b ~)", 5, false).unwrap();
        assert_eq!(c.query_string, r#"("a b"~5)"#);
    }

    #[test]
    fn error_position_is_one_based() {
        let err = compile("a&&b", 20, true).unwrap_err();
        assert_eq!(error_text(&err), "不可有連續的邏輯運算子（第 3 個字元）");
    }

    #[test]
    fn diff_merges_neighbouring_changes() {
        let d = diff("咖啡&拿鐵", "咖啡|拿鐵!廣告");
        let ops: Vec<(&str, &str)> = d.iter().map(|s| (s.op, s.text.as_str())).collect();
        assert_eq!(
            ops,
            [
                ("equal", "咖啡"),
                ("delete", "&"),
                ("insert", "|"),
                ("equal", "拿鐵"),
                ("insert", "!廣告")
            ]
        );
        assert_eq!(
            diff("a", "a"),
            [Segment {
                op: "equal",
                text: "a".into()
            }]
        );
    }
}

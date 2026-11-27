use es_keyword_query::{validate, Comma, ErrorKind, Joiner, Options, Query};

fn qs(input: &str) -> String {
    Query::parse(input).unwrap().to_query_string()
}

fn raw(input: &str) -> String {
    let options = Options {
        precise: false,
        ..Options::default()
    };
    Query::parse_with(input, options).unwrap().to_query_string()
}

fn kind(input: &str) -> ErrorKind {
    Query::parse(input).unwrap_err().kind
}

#[test]
fn operators() {
    assert_eq!(qs("珍珠奶茶"), r#""珍珠奶茶""#);
    assert_eq!(qs("a&b"), r#""a" AND "b""#);
    assert_eq!(qs("a|b"), r#""a" OR "b""#);
    assert_eq!(qs("a!b"), r#""a" NOT "b""#);
    assert_eq!(qs("a&!b"), r#""a" AND NOT "b""#);
    assert_eq!(qs("a b"), r#""a b""#);
    assert_eq!(qs("  a  "), r#""a""#);
}

#[test]
fn groups() {
    assert_eq!(qs("(a|b)&c"), r#"("a" OR "b") AND "c""#);
    assert_eq!(qs("a&(b|c)!d"), r#""a" AND ("b" OR "c") NOT "d""#);
    assert_eq!(qs("a!(b)"), r#""a" NOT ("b")"#);
    assert_eq!(
        qs("((a|b)&(c|d))!(e|f)"),
        r#"(("a" OR "b") AND ("c" OR "d")) NOT ("e" OR "f")"#
    );
    assert_eq!(qs("(!a)&b"), r#"(NOT "a") AND "b""#);
}

#[test]
fn proximity() {
    assert_eq!(qs("(珍珠 鮮奶 ~)"), r#"("珍珠 鮮奶"~20)"#);
    assert_eq!(qs("(a b ~3)"), r#"("a b"~3)"#);
    assert_eq!(qs("(a b ~ 3)"), r#"("a b"~3)"#);
    assert_eq!(qs("(a b ~ )"), r#"("a b"~20)"#);
    assert_eq!(qs("(a b~)"), r#"("a b"~20)"#);
    assert_eq!(qs("(a|b c ~)"), r#"("a" OR "b c"~20)"#);
    assert_eq!(qs("(a b ~)&(c d ~)"), r#"("a b"~20) AND ("c d"~20)"#);
    assert_eq!(qs("a b ~5"), r#""a b"~5"#);
    assert_eq!(
        qs("(珍珠奶茶|(珍珠 鮮奶 ~)|紅茶拿鐵)"),
        r#"("珍珠奶茶" OR ("珍珠 鮮奶"~20) OR "紅茶拿鐵")"#
    );
}

#[test]
fn default_slop_is_configurable() {
    let options = Options {
        default_slop: 8,
        ..Options::default()
    };
    let q = Query::parse_with("(a b ~)", options).unwrap();
    assert_eq!(q.to_query_string(), r#"("a b"~8)"#);
}

#[test]
fn escapes() {
    assert_eq!(qs(r"a\&b"), r#""a&b""#);
    assert_eq!(qs(r"a\|b&c"), r#""a|b" AND "c""#);
    assert_eq!(qs(r"AT\&T|M\&M"), r#""AT&T" OR "M&M""#);
    assert_eq!(qs(r"smile \(:\)"), r#""smile (:)""#);
    assert_eq!(qs(r"C:\dir"), r#""C:\\dir""#);
    assert_eq!(qs(r#"say "hi""#), r#""say \"hi\"""#);
}

#[test]
fn full_width_input_is_normalised() {
    assert_eq!(qs("Ｃ＆Ｄ"), r#""C" AND "D""#);
    assert_eq!(qs("ａ｜ｂ"), r#""a" OR "b""#);
    assert_eq!(qs("（ａ｜ｂ）！ｃ"), r#"("a" OR "b") NOT "c""#);
    assert_eq!(qs("a\u{3000}b"), r#""a b""#);
    assert_eq!(qs("a\u{1}b\tc"), r#""abc""#);
}

#[test]
fn raw_mode_leaves_terms_unquoted() {
    assert_eq!(raw("iphone*&(b|c)"), "iphone* AND (b OR c)");
    assert_eq!(raw("title:a|b"), "title:a OR b");
    assert_eq!(raw("(a b ~)"), r#"("a b"~20)"#);
}

#[test]
fn commas() {
    assert_eq!(qs("a,b"), r#""a,b""#);

    let join = |joiner| Options {
        comma: Comma::Join(joiner),
        ..Options::default()
    };
    let q = Query::parse_with("a , b|c", join(Joiner::Or)).unwrap();
    assert_eq!(q.to_query_string(), r#"("a") OR ("b" OR "c")"#);
    let q = Query::parse_with("a,b", join(Joiner::And)).unwrap();
    assert_eq!(q.to_query_string(), r#"("a") AND ("b")"#);
    let q = Query::parse_with("a&b", join(Joiner::And)).unwrap();
    assert_eq!(q.to_query_string(), r#""a" AND "b""#);

    let err = |input, comma| {
        let options = Options {
            comma,
            ..Options::default()
        };
        Query::parse_with(input, options).unwrap_err().kind
    };
    assert_eq!(err("a,b", Comma::Reject), ErrorKind::Comma);
    assert_eq!(err("(a,b)", Comma::Join(Joiner::Or)), ErrorKind::Comma);
    assert_eq!(err("a,,b", Comma::Join(Joiner::Or)), ErrorKind::Empty);
    assert_eq!(err("a,", Comma::Join(Joiner::Or)), ErrorKind::Empty);
}

#[test]
fn terms_and_filter() {
    let q = Query::parse("(a|(b c ~))&d!e").unwrap();
    assert_eq!(q.terms(), ["a", "b c", "d", "e"]);

    let filter = Query::parse("x|y").unwrap();
    assert_eq!(
        q.filtered_by(&filter),
        r#"("x" OR "y") AND (("a" OR ("b c"~20)) AND "d" NOT "e")"#
    );
}

#[test]
fn syntax_errors() {
    assert_eq!(kind(""), ErrorKind::Empty);
    assert_eq!(kind("   "), ErrorKind::Empty);
    assert_eq!(kind("()"), ErrorKind::Empty);
    assert_eq!(kind("a&( )"), ErrorKind::Empty);
    assert_eq!(kind("**"), ErrorKind::ConsecutiveWildcards);
    assert_eq!(kind("a**"), ErrorKind::ConsecutiveWildcards);
    assert_eq!(kind("*"), ErrorKind::LoneWildcard);
    assert_eq!(kind("a&*"), ErrorKind::LoneWildcard);
    assert_eq!(kind("(a|*)"), ErrorKind::LoneWildcard);
    assert_eq!(kind("a&&b"), ErrorKind::ConsecutiveOperators);
    assert_eq!(kind("a&|b"), ErrorKind::ConsecutiveOperators);
    assert_eq!(kind("a!!b"), ErrorKind::ConsecutiveOperators);
    assert_eq!(kind("!a"), ErrorKind::DanglingOperator);
    assert_eq!(kind("&a"), ErrorKind::DanglingOperator);
    assert_eq!(kind("a&"), ErrorKind::DanglingOperator);
    assert_eq!(kind("a!"), ErrorKind::DanglingOperator);
    assert_eq!(kind("(a|)"), ErrorKind::DanglingOperator);
    assert_eq!(kind("((a)"), ErrorKind::UnbalancedParentheses);
    assert_eq!(kind("a)("), ErrorKind::UnbalancedParentheses);
    assert_eq!(kind("(a)b"), ErrorKind::MissingOperator);
    assert_eq!(kind("a ~ b"), ErrorKind::MissingOperator);
    assert_eq!(kind("~a"), ErrorKind::BadProximity);
    assert_eq!(kind("(a|b)~"), ErrorKind::BadProximity);
    assert_eq!(kind("a~~"), ErrorKind::BadProximity);
    assert_eq!(kind("a&~"), ErrorKind::BadProximity);
    assert_eq!(kind("a~99999999999"), ErrorKind::BadProximity);
}

#[test]
fn error_reports_position_and_messages() {
    let err = Query::parse("ab&&c").unwrap_err();
    assert_eq!(err.position, 3);
    assert_eq!(err.message_zh(), "不可有連續的邏輯運算子");
    assert_eq!(
        err.to_string(),
        "consecutive operators are not allowed (at character 3)"
    );
    assert!(validate("a|b").is_ok());
    assert!(validate("a|").is_err());
}

#[test]
fn grouped_filter_does_not_leak_or_into_the_query() {
    let q = Query::parse("tea|coffee").unwrap();
    let filter = Query::parse("shop|cafe").unwrap();
    assert_eq!(
        q.filtered_by(&filter),
        r#"("shop" OR "cafe") AND ("tea" OR "coffee")"#
    );
}

#[test]
fn nesting_limit_rejects_adversarial_input_without_recursing_unboundedly() {
    let accepted = format!("{}tea{}", "(".repeat(128), ")".repeat(128));
    assert_eq!(Query::parse(&accepted).unwrap().terms(), ["tea"]);
    let rejected = format!("{}tea{}", "(".repeat(10_000), ")".repeat(10_000));
    let error = Query::parse(&rejected).unwrap_err();
    assert_eq!(error.kind, ErrorKind::NestingTooDeep);
    assert_eq!(error.position, 129);
}

#[test]
fn reference_examples_match_the_compiler() {
    for (input, output) in [
        (
            "(tea|coffee)&shop!ad",
            r#"("tea" OR "coffee") AND "shop" NOT "ad""#,
        ),
        ("(apple pie ~5)&bakery", r#"("apple pie"~5) AND "bakery""#),
        ("（茶｜咖啡）＆店", r#"("茶" OR "咖啡") AND "店""#),
    ] {
        assert_eq!(Query::parse(input).unwrap().to_query_string(), output);
    }
}

#[test]
fn clauses_preserve_structure_without_inventing_precedence() {
    use es_keyword_query::ClauseKind;
    let q = Query::parse("(tea|coffee)&shop!(paid ad ~5)").unwrap();
    let nodes = q.clauses();
    assert_eq!(nodes.len(), 6);
    assert_eq!(nodes[0].kind, ClauseKind::Group);
    assert_eq!(nodes[1].parent, Some(0));
    assert_eq!(nodes[2].parent, Some(0));
    assert_eq!(nodes[2].joiner, Some(Joiner::Or));
    assert_eq!(nodes[3].joiner, Some(Joiner::And));
    assert_eq!(nodes[3].parent, None);
    assert!(nodes[4].negated);
    assert_eq!(nodes[5].parent, Some(4));
    assert_eq!(nodes[5].slop, Some(5));
    assert_eq!(nodes[5].output.as_deref(), Some(r#""paid ad"~5"#));
    assert!(nodes[0].output.is_none());
}

#[test]
fn clauses_use_normalized_terms_and_the_actual_output_options() {
    use es_keyword_query::ClauseKind;
    let q = Query::parse_with(
        "（ＡＴ\\＆Ｔ｜tea）&(apple pie ~)",
        Options {
            precise: false,
            default_slop: 9,
            ..Options::default()
        },
    )
    .unwrap();
    let nodes = q.clauses();
    assert_eq!(nodes[1].text.as_deref(), Some("AT&T"));
    assert_eq!(nodes[1].output.as_deref(), Some("AT&T"));
    assert_eq!(nodes[4].kind, ClauseKind::Proximity);
    assert_eq!(nodes[4].slop, Some(9));
    assert_eq!(nodes[4].output.as_deref(), Some(r#""apple pie"~9"#));
}

use keyword_vault::workbench::{inspect, InspectInput};
use serde_json::{json, Value};

fn run(value: Value) -> Value {
    inspect(serde_json::from_value::<InspectInput>(value).unwrap(), 20).unwrap()
}

#[test]
fn filter_groups_both_sides_and_retains_authored_sources() {
    let result = run(json!({"expression":"tea|coffee", "filter_expression":"shop|cafe"}));
    let artifact = &result["inspection"];
    assert_eq!(
        artifact["query_string"],
        r#"("shop" OR "cafe") AND ("tea" OR "coffee")"#
    );
    assert_eq!(artifact["expression"], "tea|coffee");
    assert_eq!(artifact["filter_expression"], "shop|cafe");
    assert_eq!(artifact["query"]["terms"], json!(["tea", "coffee"]));
    assert_eq!(artifact["filter"]["terms"], json!(["shop", "cafe"]));
}

#[test]
fn options_do_not_mutate_the_source_and_blank_filter_is_absent() {
    let result = run(
        json!({"expression":"(珍珠 鮮奶 ~)&tea*", "filter_expression":"  ", "precise":false, "default_slop":5}),
    );
    assert_eq!(
        result["inspection"]["query_string"],
        r#"("珍珠 鮮奶"~5) AND tea*"#
    );
    assert_eq!(result["inspection"]["expression"], "(珍珠 鮮奶 ~)&tea*");
    assert!(result["inspection"]["filter"].is_null());
    assert_eq!(
        result["inspection"]["options"],
        json!({"precise":false,"default_slop":5})
    );
}

#[test]
fn filter_syntax_error_names_its_source_without_stale_output() {
    let result = run(json!({"expression":"tea", "filter_expression":"a&&b"}));
    assert_eq!(result["valid"], false);
    assert_eq!(result["error"]["source"], "filter");
    assert_eq!(result["error"]["position"], 2);
    assert!(result.get("inspection").is_none());
}

#[test]
fn combined_scalar_length_is_bounded_and_options_are_typed() {
    let input = serde_json::from_value(
        json!({"expression":"茶".repeat(25_001),"filter_expression":"茶".repeat(25_000)}),
    )
    .unwrap();
    assert!(inspect(input, 20).is_err());
    assert!(
        serde_json::from_value::<InspectInput>(json!({"expression":"tea","default_slop":-1}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<InspectInput>(json!({"expression":"tea","unknown":true})).is_err()
    );
}

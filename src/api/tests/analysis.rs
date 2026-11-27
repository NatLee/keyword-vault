use keyword_vault::analysis::{AnalysisInput, Analyzer};
use serde_json::{json, Value};
use std::fs;

fn input(value: Value) -> AnalysisInput {
    serde_json::from_value(value).unwrap()
}

#[test]
fn traditional_dictionary_and_scalar_offsets_preserve_leaf_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("user.txt");
    fs::write(&path, "臺積電 100000000 nz\n𠮷野家 100000000 nz\n").unwrap();
    let analyzer = Analyzer::load(Some(&path)).unwrap();
    let result = serde_json::to_value(
        analyzer
            .analyze(input(
                json!({"expression":"臺積電＆𠮷野家🍵!(廣告|（珍珠 鮮奶 ～5）)"}),
            ))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result["offset_unit"], "unicode_scalar");
    assert_eq!(result["mode"], "normal");
    assert_eq!(result["leaves"][0]["tokens"][0]["text"], "臺積電");
    for leaf in result["leaves"].as_array().unwrap() {
        let chars: Vec<char> = leaf["text"].as_str().unwrap().chars().collect();
        for token in leaf["tokens"].as_array().unwrap() {
            let start = token["start"].as_u64().unwrap() as usize;
            let end = token["end"].as_u64().unwrap() as usize;
            assert_eq!(
                chars[start..end].iter().collect::<String>(),
                token["text"].as_str().unwrap()
            );
        }
    }
    assert_ne!(result["dictionary"]["sha256"], "");
}

#[test]
fn raw_lucene_references_invalid_syntax_and_oversized_input_are_rejected() {
    let analyzer = Analyzer::load(None).unwrap();
    for value in [
        json!({"expression":"title:茶*","precise":false}),
        json!({"expression":"#1|#2"}),
        json!({"expression":"a&&b"}),
        json!({"expression":"茶".repeat(10_001)}),
    ] {
        assert!(analyzer.analyze(input(value)).is_err());
    }
    assert!(
        serde_json::from_value::<AnalysisInput>(json!({"expression":"茶","mode":"all"})).is_err()
    );
}

#[test]
fn search_mode_is_explicit_and_operator_text_is_not_a_leaf() {
    let analyzer = Analyzer::load(None).unwrap();
    let result = serde_json::to_value(
        analyzer
            .analyze(input(
                json!({"expression":"(珍珠奶茶|台灣高鐵)!廣告","mode":"search"}),
            ))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result["mode"], "search");
    assert_eq!(result["leaves"].as_array().unwrap().len(), 3);
    assert_eq!(result["leaves"][0]["clause"], 1);
    assert_eq!(result["leaves"][2]["text"], "廣告");
}

#[test]
fn malformed_or_unbounded_custom_dictionaries_fail_before_activation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.txt");
    for text in [
        "詞 0".to_owned(),
        "詞 1000000001".to_owned(),
        "詞 nope".to_owned(),
        "詞 1\n詞 2".to_owned(),
        format!("{} 1", "詞".repeat(101)),
        "詞 1 tag extra".to_owned(),
        "x".repeat(1_048_577),
    ] {
        fs::write(&path, text).unwrap();
        assert!(Analyzer::load(Some(&path)).is_err());
    }
    fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(Analyzer::load(Some(&path)).is_err());
}

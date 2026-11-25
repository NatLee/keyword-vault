//! 組合關鍵字：用其他關鍵字的編號組出新的關鍵字，例如 `(#3|#5)!#8`。
//!
//! 展開時每個編號換成 `(那個關鍵字的語法)`，所以被引用的關鍵字內部的運算子
//! 不會和外面的混在一起。

use std::collections::HashMap;

use es_keyword_query::Query;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Id(i64),
    Op(char),
}

/// 解析過的組合方式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logic {
    tokens: Vec<Token>,
}

impl Logic {
    /// 解析 `(#3|#5)!#8`。`#` 可以省略，全形的運算子和括號也接受。
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut tokens = Vec::new();
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            let c = match c {
                '＆' => '&',
                '｜' => '|',
                '！' => '!',
                '（' => '(',
                '）' => ')',
                '＃' => '#',
                other => other,
            };
            match c {
                c if c.is_whitespace() => {}
                '&' | '|' | '!' | '(' | ')' => tokens.push(Token::Op(c)),
                '#' | '0'..='9' => {
                    let mut digits = String::new();
                    if c != '#' {
                        digits.push(c);
                    }
                    while let Some(d) = chars.peek().copied().filter(char::is_ascii_digit) {
                        digits.push(d);
                        chars.next();
                    }
                    let id = digits
                        .parse::<i64>()
                        .ok()
                        .filter(|id| *id > 0)
                        .ok_or("# 後面要接關鍵字的編號")?;
                    tokens.push(Token::Id(id));
                }
                other => {
                    return Err(format!(
                        "組合方式只能用關鍵字編號、& | ! 和括號，不能有「{other}」"
                    ))
                }
            }
        }
        // 兩個編號直接相連（`#1 #2`）在骨架裡會黏成一個詞，要先擋掉
        if tokens
            .windows(2)
            .any(|w| matches!(w, [Token::Id(_), Token::Id(_)]))
        {
            return Err("關鍵字之間缺少邏輯運算子".into());
        }
        let logic = Self { tokens };
        // 把編號換成一個普通的詞，借用語法解析器檢查運算子和括號
        let skeleton: String = logic
            .tokens
            .iter()
            .map(|t| match t {
                Token::Id(_) => 'k',
                Token::Op(c) => *c,
            })
            .collect();
        if let Err(e) = Query::parse(&skeleton) {
            return Err(e.message_zh().to_owned());
        }
        if logic.ids().len() < 2 {
            return Err("組合關鍵字至少要用到兩個不同的關鍵字".into());
        }
        Ok(logic)
    }

    /// 用到的關鍵字編號，照出現順序，不重複。
    pub fn ids(&self) -> Vec<i64> {
        let mut ids = Vec::new();
        for token in &self.tokens {
            if let Token::Id(id) = token {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        ids
    }

    /// 存進資料庫的寫法：`(#3|#5)!#8`。
    pub fn canonical(&self) -> String {
        self.tokens
            .iter()
            .map(|t| match t {
                Token::Id(id) => format!("#{id}"),
                Token::Op(c) => c.to_string(),
            })
            .collect()
    }

    /// 把每個編號換成 `(語法)`。`expressions` 要有用到的每個編號。
    pub fn expand(&self, expressions: &HashMap<i64, String>) -> String {
        let mut out = String::new();
        for token in &self.tokens {
            match token {
                Token::Id(id) => {
                    out.push('(');
                    out.push_str(expressions.get(id).map(String::as_str).unwrap_or_default());
                    out.push(')');
                }
                Token::Op(c) => out.push(*c),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exprs(pairs: &[(i64, &str)]) -> HashMap<i64, String> {
        pairs.iter().map(|(k, v)| (*k, v.to_string())).collect()
    }

    #[test]
    fn parses_and_normalises() {
        let logic = Logic::parse(" ( 3 ｜ #5 ) ！ ＃8 ").unwrap();
        assert_eq!(logic.canonical(), "(#3|#5)!#8");
        assert_eq!(logic.ids(), [3, 5, 8]);
        assert_eq!(Logic::parse("#3&#5&#3").unwrap().ids(), [3, 5]);
    }

    #[test]
    fn expansion_keeps_each_keyword_in_its_own_parentheses() {
        // 舊版直接把語法接起來，「a|b & c」的意思會跑掉
        let logic = Logic::parse("#1&#2").unwrap();
        assert_eq!(
            logic.expand(&exprs(&[(1, "a|b"), (2, "c!d")])),
            "(a|b)&(c!d)"
        );
        let logic = Logic::parse("(#1|#2)!#3").unwrap();
        let expanded = logic.expand(&exprs(&[(1, "a"), (2, "b c ~"), (3, "x|y")]));
        assert_eq!(expanded, "((a)|(b c ~))!(x|y)");
        assert!(Query::parse(&expanded).is_ok());
    }

    #[test]
    fn rejects_bad_logic() {
        assert!(Logic::parse("").is_err());
        assert!(Logic::parse("#1").is_err(), "只有一個關鍵字");
        assert!(Logic::parse("#1&#1").is_err(), "同一個關鍵字用兩次");
        assert!(Logic::parse("#1&&#2").is_err());
        assert!(Logic::parse("(#1|#2").is_err());
        assert!(Logic::parse("#1 #2").is_err(), "中間沒有運算子");
        assert!(Logic::parse("#1&abc").is_err());
        assert!(Logic::parse("#&#2").is_err());
        assert!(Logic::parse("#0&#2").is_err());
    }
}

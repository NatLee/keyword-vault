use std::fmt;

/// What is wrong with a keyword expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// More than 128 nested groups.
    NestingTooDeep,
    /// The input, or a pair of parentheses, has nothing in it.
    Empty,
    /// A comma was used while [`Comma::Reject`](crate::Comma::Reject) is set,
    /// or inside parentheses while [`Comma::Join`](crate::Comma::Join) is set.
    Comma,
    /// `**`
    ConsecutiveWildcards,
    /// Two operators with nothing between them, such as `a&&b` or `a&|b`.
    ConsecutiveOperators,
    /// The expression starts or ends with an operator.
    DanglingOperator,
    /// A term that is only `*`.
    LoneWildcard,
    /// Parentheses do not pair up.
    UnbalancedParentheses,
    /// Two operands with no operator between them, such as `(a)b` or `a ~ b`.
    MissingOperator,
    /// `~` does not follow a term, or its slop is not a valid number.
    BadProximity,
}

/// A syntax error, with the character position where it was noticed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    /// Zero-based character offset into the NFKC-normalised, trimmed input.
    pub position: usize,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, position: usize) -> Self {
        Self { kind, position }
    }

    /// The message in Traditional Chinese.
    pub fn message_zh(&self) -> &'static str {
        match self.kind {
            ErrorKind::NestingTooDeep => "括號巢狀深度不可超過 128 層",
            ErrorKind::Empty => "禁止空字串",
            ErrorKind::Comma => "不可使用逗點",
            ErrorKind::ConsecutiveWildcards => "不可有連續的萬用字元(*)",
            ErrorKind::ConsecutiveOperators => "不可有連續的邏輯運算子",
            ErrorKind::DanglingOperator => "不可以邏輯運算子開始或結束",
            ErrorKind::LoneWildcard => "不可使用單一的萬用字元(*)",
            ErrorKind::UnbalancedParentheses => "不對稱的括號",
            ErrorKind::MissingOperator => "關鍵字之間缺少邏輯運算子",
            ErrorKind::BadProximity => "鄰近運算子(~)必須接在關鍵字之後",
        }
    }

    /// The message in English.
    pub fn message(&self) -> &'static str {
        match self.kind {
            ErrorKind::NestingTooDeep => "parentheses nesting exceeds 128 levels",
            ErrorKind::Empty => "the query is empty",
            ErrorKind::Comma => "a comma is not allowed here",
            ErrorKind::ConsecutiveWildcards => "consecutive wildcards (*) are not allowed",
            ErrorKind::ConsecutiveOperators => "consecutive operators are not allowed",
            ErrorKind::DanglingOperator => "the query cannot start or end with an operator",
            ErrorKind::LoneWildcard => "a wildcard (*) cannot be used on its own",
            ErrorKind::UnbalancedParentheses => "parentheses are not balanced",
            ErrorKind::MissingOperator => "an operator is missing between two keywords",
            ErrorKind::BadProximity => "the proximity operator (~) must follow a keyword",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at character {})", self.message(), self.position)
    }
}

impl std::error::Error for Error {}

//! A compact boolean keyword syntax that compiles to an Elasticsearch /
//! OpenSearch `query_string`.
//!
//! | Syntax | Meaning | Output |
//! |---|---|---|
//! | `a&b` | both | `"a" AND "b"` |
//! | `a\|b` | either | `"a" OR "b"` |
//! | `a!b` | first without second | `"a" NOT "b"` |
//! | `(a b ~)` | words within 20 positions of each other | `("a b"~20)` |
//! | `(a b ~5)` | words within 5 positions | `("a b"~5)` |
//! | `\&` `\|` `\!` `\~` `\(` `\)` `\\` | the literal character | |
//!
//! Input is NFKC-normalised first, so full-width operators such as `＆` and
//! `｜` work as well.
//!
//! ```
//! use es_keyword_query::Query;
//!
//! let q = Query::parse("(apple|(apple pie ~))&store!rumor").unwrap();
//! assert_eq!(
//!     q.to_query_string(),
//!     r#"("apple" OR ("apple pie"~20)) AND "store" NOT "rumor""#
//! );
//! assert_eq!(q.terms(), ["apple", "apple pie", "store", "rumor"]);
//! ```
//!
//! Operators are emitted in the order they were written and no precedence is
//! added: `a|b&c` becomes `"a" OR "b" AND "c"`, which Lucene reads with its own
//! rules. Use parentheses whenever `&` and `|` are mixed.

mod error;
mod parser;

pub use error::{Error, ErrorKind};

/// How a comma in the input is treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Comma {
    /// A comma is an ordinary character inside a term.
    #[default]
    Literal,
    /// A comma is a syntax error.
    Reject,
    /// Top-level commas split the input into parts, each wrapped in
    /// parentheses and joined with this operator.
    Join(Joiner),
}

/// The operator that joins comma-separated parts under [`Comma::Join`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Joiner {
    And,
    Or,
}

/// The kind of a node in an inspected query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseKind {
    /// An ordinary term, including literal escaped operators.
    Term,
    /// A phrase with a slop value.
    Proximity,
    /// An explicit group (or a group introduced by comma splitting).
    Group,
}

/// One node of a query, returned in preorder by [`Query::clauses`].
///
/// This describes written structure, not the search engine's Boolean precedence.
/// `negated` belongs to this node; children of an excluded group do not repeat it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    /// Index of the containing group in the returned list; `None` at the root.
    pub parent: Option<usize>,
    /// The node's kind.
    pub kind: ClauseKind,
    /// Operator before this node; exclusions may have no explicit joiner.
    pub joiner: Option<Joiner>,
    /// Whether `!` directly excludes this node.
    pub negated: bool,
    /// Normalized, unescaped term or phrase. Groups have no text.
    pub text: Option<String>,
    /// Resolved phrase slop, including the configured default.
    pub slop: Option<u32>,
    /// Exact compiled leaf text, without its joiner or exclusion. Groups have none.
    pub output: Option<String>,
}

/// Parsing and output options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Wrap every term in double quotes so it matches as an exact phrase.
    /// When `false`, terms are written as typed and Lucene syntax inside them
    /// (wildcards, field names) passes through.
    pub precise: bool,
    /// The slop used for `~` without a number.
    pub default_slop: u32,
    /// How commas are treated.
    pub comma: Comma,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            precise: true,
            default_slop: 20,
            comma: Comma::Literal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Node {
    Term(String),
    Near { phrase: String, slop: Option<u32> },
    Group(Vec<Item>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Link {
    /// The first item of a sequence, or one joined only by `!`.
    None,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Item {
    pub link: Link,
    pub negated: bool,
    pub node: Node,
}

/// A parsed keyword expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    items: Vec<Item>,
    options: Options,
}

impl Query {
    /// Parses `input` with the default [`Options`].
    pub fn parse(input: &str) -> Result<Self, Error> {
        Self::parse_with(input, Options::default())
    }

    /// Parses `input` with the given options.
    pub fn parse_with(input: &str, options: Options) -> Result<Self, Error> {
        let items = parser::parse(input, &options)?;
        Ok(Self { items, options })
    }

    /// Returns the `query_string` text for Elasticsearch / OpenSearch.
    pub fn to_query_string(&self) -> String {
        let mut out = String::new();
        self.write_items(&self.items, &mut out);
        out
    }

    /// Returns the query restricted by `filter`: `(<filter>) AND (<self>)`.
    pub fn filtered_by(&self, filter: &Query) -> String {
        format!(
            "({}) AND ({})",
            filter.to_query_string(),
            self.to_query_string()
        )
    }

    /// Returns every term and proximity phrase in the order written,
    /// negated ones included.
    pub fn terms(&self) -> Vec<&str> {
        fn walk<'a>(items: &'a [Item], out: &mut Vec<&'a str>) {
            for item in items {
                match &item.node {
                    Node::Term(t) => out.push(t),
                    Node::Near { phrase, .. } => out.push(phrase),
                    Node::Group(inner) => walk(inner, out),
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.items, &mut out);
        out
    }

    /// Inspects explicit grouping and ordered clauses without parsing again.
    ///
    /// Parent indices always precede their children. Leaf output uses this query's
    /// precise/raw and default-slop options. No inferred operator precedence is added.
    pub fn clauses(&self) -> Vec<Clause> {
        fn walk(query: &Query, items: &[Item], parent: Option<usize>, out: &mut Vec<Clause>) {
            for item in items {
                let (kind, text, slop) = match &item.node {
                    Node::Group(_) => (ClauseKind::Group, None, None),
                    Node::Term(text) => (ClauseKind::Term, Some(text.clone()), None),
                    Node::Near { phrase, slop } => (
                        ClauseKind::Proximity,
                        Some(phrase.clone()),
                        Some(slop.unwrap_or(query.options.default_slop)),
                    ),
                };
                let output = if kind == ClauseKind::Group {
                    None
                } else {
                    let mut text = String::new();
                    query.write_node(&item.node, &mut text);
                    Some(text)
                };
                let index = out.len();
                out.push(Clause {
                    parent,
                    kind,
                    text,
                    slop,
                    output,
                    negated: item.negated,
                    joiner: match item.link {
                        Link::None => None,
                        Link::And => Some(Joiner::And),
                        Link::Or => Some(Joiner::Or),
                    },
                });
                if let Node::Group(children) = &item.node {
                    walk(query, children, Some(index), out);
                }
            }
        }
        let mut out = Vec::new();
        walk(self, &self.items, None, &mut out);
        out
    }

    fn write_items(&self, items: &[Item], out: &mut String) {
        for (i, item) in items.iter().enumerate() {
            match (item.link, item.negated) {
                (Link::None, false) => {}
                (Link::None, true) if i == 0 => out.push_str("NOT "),
                (Link::None, true) => out.push_str(" NOT "),
                (Link::And, false) => out.push_str(" AND "),
                (Link::And, true) => out.push_str(" AND NOT "),
                (Link::Or, false) => out.push_str(" OR "),
                (Link::Or, true) => out.push_str(" OR NOT "),
            }
            self.write_node(&item.node, out);
        }
    }

    fn write_node(&self, node: &Node, out: &mut String) {
        match node {
            Node::Term(t) => self.write_term(t, out),
            Node::Near { phrase, slop } => {
                write_quoted(phrase, out);
                out.push('~');
                out.push_str(&slop.unwrap_or(self.options.default_slop).to_string());
            }
            Node::Group(inner) => {
                out.push('(');
                self.write_items(inner, out);
                out.push(')');
            }
        }
    }

    fn write_term(&self, term: &str, out: &mut String) {
        if self.options.precise {
            write_quoted(term, out);
        } else {
            out.push_str(term);
        }
    }
}

fn write_quoted(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
}

/// Checks the syntax of `input` with the default [`Options`].
pub fn validate(input: &str) -> Result<(), Error> {
    Query::parse(input).map(|_| ())
}

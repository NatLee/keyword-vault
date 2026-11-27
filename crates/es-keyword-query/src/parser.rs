use unicode_normalization::UnicodeNormalization;

use crate::{Comma, Error, ErrorKind, Item, Joiner, Link, Node, Options};

// Bound recursion for untrusted expressions; accepted trees are also safe to walk/drop.
const MAX_DEPTH: usize = 128;

const ESCAPABLE: &[char] = &['&', '|', '!', '~', '(', ')', '\\'];

pub(crate) fn parse(input: &str, options: &Options) -> Result<Vec<Item>, Error> {
    let normalized: String = input.nfkc().collect();
    let chars: Vec<char> = normalized
        .trim()
        .chars()
        .filter(|c| !('\u{1}'..='\u{1a}').contains(c))
        .collect();
    let mut parser = Parser {
        chars,
        pos: 0,
        comma: options.comma,
    };

    let mut parts = Vec::new();
    loop {
        let (items, end) = parser.sequence(0)?;
        parts.push(items);
        match end {
            End::Comma => continue,
            End::Eof => break,
            End::Close => unreachable!("a top-level `)` is reported by sequence()"),
        }
    }
    if parts.len() == 1 {
        return Ok(parts.pop().unwrap_or_default());
    }
    let link = match options.comma {
        Comma::Join(Joiner::And) => Link::And,
        _ => Link::Or,
    };
    Ok(parts
        .into_iter()
        .enumerate()
        .map(|(i, items)| Item {
            link: if i == 0 { Link::None } else { link },
            negated: false,
            node: Node::Group(items),
        })
        .collect())
}

enum End {
    Eof,
    Close,
    Comma,
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    comma: Comma,
}

impl Parser {
    fn err<T>(&self, kind: ErrorKind) -> Result<T, Error> {
        Err(Error::new(kind, self.pos))
    }

    /// Parses operands and operators until `)`, a splitting comma or the end.
    fn sequence(&mut self, depth: usize) -> Result<(Vec<Item>, End), Error> {
        if depth > MAX_DEPTH {
            return self.err(ErrorKind::NestingTooDeep);
        }
        let mut items: Vec<Item> = Vec::new();
        let mut link: Option<Link> = None;
        let mut negated = false;

        let end = loop {
            let Some(&c) = self.chars.get(self.pos) else {
                if depth > 0 {
                    return self.err(ErrorKind::UnbalancedParentheses);
                }
                break End::Eof;
            };
            match c {
                c if c.is_whitespace() => self.pos += 1,
                '&' | '|' => {
                    if link.is_some() || negated {
                        return self.err(ErrorKind::ConsecutiveOperators);
                    }
                    if items.is_empty() {
                        return self.err(ErrorKind::DanglingOperator);
                    }
                    link = Some(if c == '&' { Link::And } else { Link::Or });
                    self.pos += 1;
                }
                '!' => {
                    if negated {
                        return self.err(ErrorKind::ConsecutiveOperators);
                    }
                    // A query made only of exclusions matches nothing.
                    if items.is_empty() && depth == 0 {
                        return self.err(ErrorKind::DanglingOperator);
                    }
                    negated = true;
                    self.pos += 1;
                }
                '~' => {
                    if link.is_some() || negated {
                        return self.err(ErrorKind::BadProximity);
                    }
                    let slop = self.slop()?;
                    match items.last_mut() {
                        Some(Item { node, .. }) if matches!(node, Node::Term(_)) => {
                            let Node::Term(phrase) = std::mem::replace(node, Node::Group(vec![]))
                            else {
                                unreachable!()
                            };
                            *node = Node::Near { phrase, slop };
                        }
                        _ => return self.err(ErrorKind::BadProximity),
                    }
                }
                ')' => {
                    if depth == 0 {
                        return self.err(ErrorKind::UnbalancedParentheses);
                    }
                    self.pos += 1;
                    break End::Close;
                }
                ',' if self.comma != Comma::Literal => {
                    if self.comma == Comma::Reject || depth > 0 {
                        return self.err(ErrorKind::Comma);
                    }
                    self.pos += 1;
                    break End::Comma;
                }
                _ => {
                    if !items.is_empty() && link.is_none() && !negated {
                        return self.err(ErrorKind::MissingOperator);
                    }
                    let node = if c == '(' {
                        self.pos += 1;
                        Node::Group(self.sequence(depth + 1)?.0)
                    } else {
                        Node::Term(self.term()?)
                    };
                    items.push(Item {
                        link: link.take().unwrap_or(Link::None),
                        negated: std::mem::take(&mut negated),
                        node,
                    });
                }
            }
        };

        if link.is_some() || negated {
            return self.err(ErrorKind::DanglingOperator);
        }
        if items.is_empty() {
            return self.err(ErrorKind::Empty);
        }
        Ok((items, end))
    }

    /// Reads the optional number after `~`.
    fn slop(&mut self) -> Result<Option<u32>, Error> {
        self.pos += 1;
        let mut ahead = self.pos;
        while self.chars.get(ahead).is_some_and(|c| c.is_whitespace()) {
            ahead += 1;
        }
        let start = ahead;
        while self.chars.get(ahead).is_some_and(|c| c.is_ascii_digit()) {
            ahead += 1;
        }
        if ahead == start {
            return Ok(None);
        }
        let digits: String = self.chars[start..ahead].iter().collect();
        self.pos = ahead;
        match digits.parse() {
            Ok(n) => Ok(Some(n)),
            Err(_) => self.err(ErrorKind::BadProximity),
        }
    }

    fn term(&mut self) -> Result<String, Error> {
        let start = self.pos;
        let mut text = String::new();
        while let Some(&c) = self.chars.get(self.pos) {
            match c {
                '\\' => match self.chars.get(self.pos + 1) {
                    Some(next) if ESCAPABLE.contains(next) => {
                        text.push(*next);
                        self.pos += 2;
                    }
                    _ => {
                        text.push(c);
                        self.pos += 1;
                    }
                },
                '&' | '|' | '!' | '~' | '(' | ')' => break,
                ',' if self.comma != Comma::Literal => break,
                _ => {
                    text.push(c);
                    self.pos += 1;
                }
            }
        }
        let text = text.trim();
        if text.contains("**") {
            return Err(Error::new(ErrorKind::ConsecutiveWildcards, start));
        }
        if text == "*" {
            return Err(Error::new(ErrorKind::LoneWildcard, start));
        }
        Ok(text.to_owned())
    }
}

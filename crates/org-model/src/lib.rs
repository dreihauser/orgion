//! Domain types shared by `org-parser`, `org-storage`, `org-index`, and
//! `apps/server`. See `docs/data-model.md` for the authoritative field
//! documentation — this module is the Rust encoding of that document.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Whether a TODO keyword counts as "not done" or "done" for agenda /
/// board-grouping purposes. Derived from which side of the `|` it sits on
/// in a `#+TODO:` line (see docs/org-mapping.md §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TodoType {
    Todo,
    Done,
}

/// A single keyword in a `#+TODO:` sequence, e.g. `NEXT(n)` -> `{keyword:
/// "NEXT", shortcut: Some('n')}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoKeyword {
    pub keyword: String,
    pub todo_type: TodoType,
}

/// The full ordered TODO keyword vocabulary for a file (or the workspace
/// default when a file declares none). Order matters: it is the order
/// keywords cycle through in the UI's status dropdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoKeywords(pub Vec<TodoKeyword>);

impl Default for TodoKeywords {
    fn default() -> Self {
        TodoKeywords(vec![
            TodoKeyword {
                keyword: "TODO".into(),
                todo_type: TodoType::Todo,
            },
            TodoKeyword {
                keyword: "DONE".into(),
                todo_type: TodoType::Done,
            },
        ])
    }
}

impl TodoKeywords {
    pub fn todo_type_of(&self, keyword: &str) -> Option<TodoType> {
        self.0
            .iter()
            .find(|k| k.keyword == keyword)
            .map(|k| k.todo_type)
    }

    pub fn is_known(&self, keyword: &str) -> bool {
        self.0.iter().any(|k| k.keyword == keyword)
    }

    /// Parse the value half of a `#+TODO: TODO(t) NEXT(n) | DONE(d)` line.
    /// Multiple `#+TODO:`/`#+SEQ_TODO:` lines in one file accumulate.
    pub fn parse_directive_value(value: &str) -> Vec<TodoKeyword> {
        let mut out = Vec::new();
        let mut todo_type = TodoType::Todo;
        for token in value.split_whitespace() {
            if token == "|" {
                todo_type = TodoType::Done;
                continue;
            }
            // Strip an optional "(x)" fast-select-key suffix.
            let keyword = match token.find('(') {
                Some(idx) => &token[..idx],
                None => token,
            };
            if keyword.is_empty() {
                continue;
            }
            out.push(TodoKeyword {
                keyword: keyword.to_string(),
                todo_type,
            });
        }
        out
    }
}

/// An org timestamp, active (`<...>`) or inactive (`[...]`), with an
/// optional time-of-day, end time, and repeater/warning cookie.
///
/// The repeater string (`+1w`, `++1d`, `.+1m`) is stored verbatim rather
/// than interpreted into a structured recurrence rule — org's repeater
/// grammar has enough edge cases (habit `.+`/`++` semantics, warning
/// periods `-2d`) that re-deriving occurrences directly from the string
/// via `org-index`'s agenda query is more faithful than re-encoding it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamp {
    pub date: chrono::NaiveDate,
    pub time: Option<chrono::NaiveTime>,
    pub end_time: Option<chrono::NaiveTime>,
    pub active: bool,
    pub repeater: Option<String>,
}

/// The `SCHEDULED:`/`DEADLINE:`/`CLOSED:` planning line under a heading.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Planning {
    pub scheduled: Option<Timestamp>,
    pub deadline: Option<Timestamp>,
    pub closed: Option<Timestamp>,
}

/// A link found in a node's body or title, e.g. `[[id:...][description]]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub target_kind: LinkKind,
    pub target_raw: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkKind {
    Id,
    File,
    Web,
    Other,
}

impl LinkKind {
    pub fn classify(raw: &str) -> (LinkKind, &str) {
        if let Some(rest) = raw.strip_prefix("id:") {
            (LinkKind::Id, rest)
        } else if let Some(rest) = raw.strip_prefix("file:") {
            (LinkKind::File, rest)
        } else if raw.starts_with("http://") || raw.starts_with("https://") {
            (LinkKind::Web, raw)
        } else {
            (LinkKind::Other, raw)
        }
    }
}

/// An org heading, materialized as a domain object. Mirrors the `Node`
/// table in docs/data-model.md. `children` is populated when this value
/// came from parsing a whole file (or subtree) into a tree; when Orgion
/// flattens nodes into index rows, `children` is dropped in favor of
/// `parent_id` foreign keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Stable identifier. `Some` only once a real `:ID:` property exists.
    pub org_id: Option<String>,
    pub level: u32,
    pub title: String,
    pub todo_state: Option<String>,
    pub todo_type: Option<TodoType>,
    pub priority: Option<char>,
    pub tags: Vec<String>,
    pub planning: Planning,
    /// Ordered to match on-disk drawer order; last-write-wins on duplicate
    /// keys, matching org's own semantics.
    pub properties: Vec<(String, String)>,
    pub links: Vec<Link>,
    /// Raw body text (between the drawer/planning line and the next
    /// heading of any level), verbatim.
    pub body: String,
    pub children: Vec<Node>,
}

impl Node {
    pub fn property(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    /// Depth-first iterator over this node and all descendants.
    pub fn iter(&self) -> NodeIter<'_> {
        NodeIter {
            stack: vec![self],
        }
    }
}

pub struct NodeIter<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a Node;
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        for child in node.children.iter().rev() {
            self.stack.push(child);
        }
        Some(node)
    }
}

/// Inferred UI type for a property key, per docs/org-mapping.md §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyUiType {
    Identifier,
    Select,
    Date,
    Number,
    Url,
    Text,
}

/// File-level `#+KEYWORD: value` lines that precede the first heading,
/// e.g. `#+TITLE:`, `#+TODO:`, `#+ORGION_VIEW:`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preamble {
    pub keywords: Vec<(String, String)>,
}

impl Preamble {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.keywords
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    pub fn get_all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> {
        self.keywords
            .iter()
            .filter(move |(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

/// A whole parsed `.org` file: preamble keywords + the top-level node tree.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub preamble: Preamble,
    pub todo_keywords: TodoKeywords,
    pub nodes: Vec<Node>,
}

impl Document {
    /// Flattened depth-first iterator over every node in the document.
    pub fn iter(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().flat_map(|n| n.iter())
    }
}

/// Simple key/value map view over a node's properties, for convenience at
/// the API/index layer. Order is not preserved (use `Node.properties` for
/// that).
pub fn properties_map(node: &Node) -> BTreeMap<String, String> {
    node.properties.iter().cloned().collect()
}

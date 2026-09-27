//! Whole-document rendering: `Document -> String`. Used for creating new
//! files and in round-trip tests. For editing an *existing* file, prefer
//! `patch.rs`, which touches only the bytes that actually changed.

use crate::heading::{render_heading_line, ParsedHeading};
use crate::planning::render_planning_line;
use org_model::{Document, Node};

pub fn render_document(doc: &Document) -> String {
    let mut out = String::new();
    for (key, value) in &doc.preamble.keywords {
        out.push_str("#+");
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push('\n');
    }
    if !doc.preamble.keywords.is_empty() && !doc.nodes.is_empty() {
        out.push('\n');
    }
    for (i, node) in doc.nodes.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        render_node(node, &mut out);
    }
    out
}

fn render_node(node: &Node, out: &mut String) {
    out.push_str(&render_header(node));
    out.push_str(&node.body);
    for child in &node.children {
        render_node(child, out);
    }
}

/// Renders just a node's heading line, optional planning line, and
/// optional properties drawer (each newline-terminated) — i.e. everything
/// that precedes the body. Shared by whole-document rendering and by
/// `patch.rs`'s targeted single-node rewrite.
pub fn render_header(node: &Node) -> String {
    let heading = ParsedHeading {
        level: node.level,
        todo_state: node.todo_state.clone(),
        priority: node.priority,
        title: node.title.clone(),
        tags: node.tags.clone(),
    };
    let mut out = String::new();
    out.push_str(&render_heading_line(&heading));
    out.push('\n');

    if let Some(line) = render_planning_line(&node.planning) {
        out.push_str(&line);
        out.push('\n');
    }

    if !node.properties.is_empty() {
        out.push_str(":PROPERTIES:\n");
        for (k, v) in &node.properties {
            out.push(':');
            out.push_str(k);
            out.push_str(": ");
            out.push_str(v);
            out.push('\n');
        }
        out.push_str(":END:\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn round_trips_semantically() {
        let src = "\
#+TITLE: Tasks
#+TODO: TODO NEXT | DONE

* TODO Build Orgion
DEADLINE: <2026-10-01 Thu>
:PROPERTIES:
:PROJECT: Orgion
:END:

* NEXT Implement parser

* DONE Create repository
";
        let pd1 = parse(src);
        let rendered = render_document(&pd1.doc);
        let pd2 = parse(&rendered);
        assert_eq!(pd1.doc.nodes.len(), pd2.doc.nodes.len());
        for (a, b) in pd1.doc.nodes.iter().zip(pd2.doc.nodes.iter()) {
            assert_eq!(a.title, b.title);
            assert_eq!(a.todo_state, b.todo_state);
            assert_eq!(a.planning, b.planning);
            assert_eq!(a.properties, b.properties);
        }
    }

    #[test]
    fn exact_byte_round_trip_for_canonical_input() {
        let src = "* TODO Build Orgion\nDEADLINE: <2026-10-01 Thu>\n:PROPERTIES:\n:PROJECT: Orgion\n:END:\n\nSome notes.\n";
        let pd = parse(src);
        assert_eq!(render_document(&pd.doc), src);
    }
}

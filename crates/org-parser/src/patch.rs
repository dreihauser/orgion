//! Targeted, minimal-diff edits to a single node within an already-parsed
//! document. Instead of re-rendering the whole file (which would
//! reformat every heading and lose the user's exact spacing/comments
//! elsewhere), this rewrites only the header region (heading line +
//! planning line + properties drawer) of the one node being changed,
//! splicing it into the original bytes.

use crate::parse::ParsedDocument;
use crate::render::render_header;
use org_model::{Node, Timestamp};
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum NodeEdit {
    SetTitle(String),
    SetTodoState(Option<String>),
    SetPriority(Option<char>),
    SetTags(Vec<String>),
    SetProperty(String, String),
    RemoveProperty(String),
    SetScheduled(Option<Timestamp>),
    SetDeadline(Option<Timestamp>),
}

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("node index {0} not found in document")]
    NodeNotFound(usize),
}

fn apply_edit_to_node(node: &mut Node, edit: &NodeEdit) {
    match edit {
        NodeEdit::SetTitle(t) => node.title = t.clone(),
        NodeEdit::SetTodoState(s) => node.todo_state = s.clone(),
        NodeEdit::SetPriority(p) => node.priority = *p,
        NodeEdit::SetTags(tags) => node.tags = tags.clone(),
        NodeEdit::SetProperty(key, value) => {
            if let Some(existing) = node
                .properties
                .iter_mut()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
            {
                existing.1 = value.clone();
            } else {
                node.properties.push((key.clone(), value.clone()));
            }
        }
        NodeEdit::RemoveProperty(key) => {
            node.properties.retain(|(k, _)| !k.eq_ignore_ascii_case(key));
        }
        NodeEdit::SetScheduled(ts) => node.planning.scheduled = ts.clone(),
        NodeEdit::SetDeadline(ts) => node.planning.deadline = ts.clone(),
    }
}

/// Apply a sequence of edits to the node at `node_index` (an index into
/// `pd.doc.iter()`'s depth-first pre-order, matching `pd.spans`) and
/// return the whole file's new text. `pd` must have been parsed from
/// exactly `text`.
pub fn apply_edits(
    text: &str,
    pd: &ParsedDocument,
    node_index: usize,
    edits: &[NodeEdit],
) -> Result<String, PatchError> {
    let node = pd
        .doc
        .iter()
        .nth(node_index)
        .ok_or(PatchError::NodeNotFound(node_index))?;
    let spans = pd
        .spans
        .get(node_index)
        .ok_or(PatchError::NodeNotFound(node_index))?;

    let mut updated = node.clone();
    for edit in edits {
        apply_edit_to_node(&mut updated, edit);
    }
    let new_header = render_header(&updated);

    let mut new_text = String::with_capacity(text.len() + new_header.len());
    new_text.push_str(&text[..spans.heading_line.start]);
    new_text.push_str(&new_header);
    new_text.push_str(&text[spans.body.start..]);
    Ok(new_text)
}

/// Convenience single-edit form of [`apply_edits`].
pub fn apply_edit(
    text: &str,
    pd: &ParsedDocument,
    node_index: usize,
    edit: NodeEdit,
) -> Result<String, PatchError> {
    apply_edits(text, pd, node_index, std::slice::from_ref(&edit))
}

/// Find the pre-order index (matching `pd.spans`) of the node whose
/// `:ID:` property equals `org_id`.
pub fn find_node_index_by_org_id(pd: &ParsedDocument, org_id: &str) -> Option<usize> {
    pd.doc
        .iter()
        .position(|n| n.org_id.as_deref() == Some(org_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    const SAMPLE: &str = "\
#+TITLE: Tasks

* TODO Build Orgion
DEADLINE: <2026-10-01 Thu>
:PROPERTIES:
:PROJECT: Orgion
:END:

Some notes.

* NEXT Implement parser
";

    #[test]
    fn todo_state_change_touches_only_that_line() {
        let pd = parse(SAMPLE);
        let new_text = apply_edit(
            SAMPLE,
            &pd,
            0,
            NodeEdit::SetTodoState(Some("DONE".to_string())),
        )
        .unwrap();
        assert!(new_text.contains("* DONE Build Orgion"));
        assert!(!new_text.contains("* TODO Build Orgion"));
        // Everything else is untouched, including the second heading.
        assert!(new_text.contains("* NEXT Implement parser"));
        assert!(new_text.contains(":PROJECT: Orgion"));
        assert!(new_text.contains("Some notes."));

        // Re-parsing the result should be self-consistent.
        let pd2 = parse(&new_text);
        assert_eq!(pd2.doc.nodes[0].todo_state.as_deref(), Some("DONE"));
        assert_eq!(pd2.doc.nodes[0].property("PROJECT"), Some("Orgion"));
    }

    #[test]
    fn adding_property_creates_drawer_when_absent() {
        let pd = parse(SAMPLE);
        let new_text = apply_edit(
            SAMPLE,
            &pd,
            1,
            NodeEdit::SetProperty("OWNER".to_string(), "keito".to_string()),
        )
        .unwrap();
        let pd2 = parse(&new_text);
        assert_eq!(pd2.doc.nodes[1].property("OWNER"), Some("keito"));
    }

    #[test]
    fn edit_by_org_id_lookup() {
        let src = "\
* TODO Write HRI Paper
:PROPERTIES:
:ID: 3a42219d-9aac-4935
:END:
";
        let pd = parse(src);
        let idx = find_node_index_by_org_id(&pd, "3a42219d-9aac-4935").unwrap();
        let new_text = apply_edit(src, &pd, idx, NodeEdit::SetTodoState(Some("DONE".into())))
            .unwrap();
        assert!(new_text.starts_with("* DONE Write HRI Paper"));
    }

    #[test]
    fn removes_all_properties_drops_drawer() {
        let pd = parse(SAMPLE);
        let new_text =
            apply_edit(SAMPLE, &pd, 0, NodeEdit::RemoveProperty("PROJECT".to_string())).unwrap();
        assert!(!new_text.contains(":PROPERTIES:"));
    }
}

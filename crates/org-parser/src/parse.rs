use crate::heading::parse_heading_line;
use crate::links::extract_links;
use crate::planning::parse_planning_line;
use org_model::{Document, Node, Planning, Preamble, TodoKeywords};

/// A byte range into the original source text. `end` is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn as_range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
    pub fn slice<'a>(&self, text: &'a str) -> &'a str {
        &text[self.start..self.end]
    }
}

/// Byte-accurate spans for one parsed node, in the source it came from.
/// Used by `org-storage` to rewrite only the lines that actually changed
/// instead of reformatting the whole file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSpans {
    pub heading_line: Span,
    pub planning_line: Option<Span>,
    pub properties_drawer: Option<Span>,
    pub body: Span,
    /// This node's heading line through the end of its last descendant
    /// (i.e. the whole subtree), used for subtree-level operations
    /// (delete, move).
    pub whole: Span,
}

/// A parsed document plus the byte spans of every node in it, in the same
/// depth-first pre-order as `Document::iter()` yields — `spans[i]`
/// corresponds to the `i`-th node from `doc.iter()`.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub doc: Document,
    pub spans: Vec<NodeSpans>,
    pub source_len: usize,
}

struct Line {
    /// Byte range of the line's content, excluding any line terminator.
    content: Span,
}

fn split_lines(text: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    // A trailing newline terminates the last line rather than starting a
    // phantom empty one, so the loop simply stops once `start` reaches
    // the end of the text.
    while start < text.len() {
        let rel_nl = bytes[start..].iter().position(|&b| b == b'\n');
        match rel_nl {
            Some(rel) => {
                let mut content_end = start + rel;
                if content_end > start && bytes[content_end - 1] == b'\r' {
                    content_end -= 1;
                }
                lines.push(Line {
                    content: Span { start, end: content_end },
                });
                start += rel + 1;
            }
            None => {
                lines.push(Line {
                    content: Span { start, end: text.len() },
                });
                break;
            }
        }
    }
    lines
}

pub fn parse(text: &str) -> ParsedDocument {
    let lines = split_lines(text);

    // Pass 0: find heading boundary lines (stars + space), independent of
    // TODO-keyword knowledge.
    let boundaries: Vec<(usize, u32)> = lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            let content = line.content.slice(text);
            let stars = content.bytes().take_while(|&b| b == b'*').count();
            if stars > 0 && content.as_bytes().get(stars) == Some(&b' ') {
                Some((idx, stars as u32))
            } else {
                None
            }
        })
        .collect();

    let first_heading_line = boundaries.first().map(|b| b.0).unwrap_or(lines.len());

    // Preamble: `#+KEY: value` lines before the first heading.
    let mut preamble = Preamble::default();
    for line in &lines[0..first_heading_line] {
        let content = line.content.slice(text);
        let trimmed = content.trim();
        if let Some(rest) = trimmed.strip_prefix("#+") {
            if let Some(colon) = rest.find(':') {
                let key = rest[..colon].to_string();
                let value = rest[colon + 1..].trim().to_string();
                preamble.keywords.push((key, value));
            }
        }
    }

    let mut todo_kw_pairs = Vec::new();
    for (key, value) in &preamble.keywords {
        if key.eq_ignore_ascii_case("TODO") || key.eq_ignore_ascii_case("SEQ_TODO") {
            todo_kw_pairs.extend(TodoKeywords::parse_directive_value(value));
        }
    }
    let todo_keywords = if todo_kw_pairs.is_empty() {
        TodoKeywords::default()
    } else {
        TodoKeywords(todo_kw_pairs)
    };

    // Pass 1: build a flat Vec<(level, Node, NodeSpans)> in source order.
    let mut flat: Vec<(u32, Node, NodeSpans)> = Vec::with_capacity(boundaries.len());

    for (bi, &(line_idx, level)) in boundaries.iter().enumerate() {
        let heading_content = lines[line_idx].content.slice(text);
        let parsed_heading = parse_heading_line(heading_content, &todo_keywords)
            .expect("boundary lines are guaranteed to be valid heading lines");

        let next_any_line = boundaries.get(bi + 1).map(|b| b.0).unwrap_or(lines.len());
        let next_same_or_higher_line = boundaries[bi + 1..]
            .iter()
            .find(|&&(_, l)| l <= level)
            .map(|&(idx, _)| idx)
            .unwrap_or(lines.len());

        let heading_line_span = lines[line_idx].content;

        let mut cursor = line_idx + 1;
        let mut planning = Planning::default();
        let mut planning_span = None;
        if cursor < next_any_line {
            let candidate = lines[cursor].content.slice(text);
            if let Some(p) = parse_planning_line(candidate) {
                planning = p;
                planning_span = Some(lines[cursor].content);
                cursor += 1;
            }
        }

        let mut properties: Vec<(String, String)> = Vec::new();
        let mut properties_span = None;
        if cursor < next_any_line && lines[cursor].content.slice(text).trim() == ":PROPERTIES:" {
            let drawer_start_line = cursor;
            let mut end_line = None;
            let mut j = cursor + 1;
            while j < next_any_line {
                if lines[j].content.slice(text).trim() == ":END:" {
                    end_line = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(end_line) = end_line {
                for pl in &lines[(drawer_start_line + 1)..end_line] {
                    let raw = pl.content.slice(text).trim();
                    if let Some(rest) = raw.strip_prefix(':') {
                        if let Some(colon2) = rest.find(':') {
                            let key = rest[..colon2].to_string();
                            let value = rest[colon2 + 1..].trim().to_string();
                            properties.push((key, value));
                        }
                    }
                }
                properties_span = Some(Span {
                    start: lines[drawer_start_line].content.start,
                    end: lines[end_line].content.end,
                });
                cursor = end_line + 1;
            }
        }

        let body_start = if cursor < next_any_line {
            lines[cursor].content.start
        } else {
            // No body lines; anchor an empty span right after the header
            // region so patches can still insert at the right place.
            heading_line_span.end
        };
        // End of body = start of the next heading line (any level), or EOF.
        let body_end = if next_any_line < lines.len() {
            lines[next_any_line].content.start
        } else {
            text.len()
        };
        let body_span = Span {
            start: body_start.min(body_end),
            end: body_end,
        };

        let title_and_body_links = {
            let mut links = extract_links(&parsed_heading.title);
            links.extend(extract_links(body_span.slice(text)));
            links
        };

        let whole_end = if next_same_or_higher_line < lines.len() {
            lines[next_same_or_higher_line].content.start
        } else {
            text.len()
        };

        let todo_type = parsed_heading
            .todo_state
            .as_deref()
            .and_then(|k| todo_keywords.todo_type_of(k));

        let node = Node {
            org_id: properties
                .iter()
                .rev()
                .find(|(k, _)| k.eq_ignore_ascii_case("ID"))
                .map(|(_, v)| v.clone()),
            level,
            title: parsed_heading.title,
            todo_state: parsed_heading.todo_state,
            todo_type,
            priority: parsed_heading.priority,
            tags: parsed_heading.tags,
            planning,
            properties,
            links: title_and_body_links,
            body: body_span.slice(text).to_string(),
            children: Vec::new(),
        };

        let spans = NodeSpans {
            heading_line: heading_line_span,
            planning_line: planning_span,
            properties_drawer: properties_span,
            body: body_span,
            whole: Span { start: heading_line_span.start, end: whole_end },
        };

        flat.push((level, node, spans));
    }

    let spans: Vec<NodeSpans> = flat.iter().map(|(_, _, s)| s.clone()).collect();
    let nodes = build_tree(flat);

    ParsedDocument {
        doc: Document {
            preamble,
            todo_keywords,
            nodes,
        },
        spans,
        source_len: text.len(),
    }
}

fn build_tree(flat: Vec<(u32, Node, NodeSpans)>) -> Vec<Node> {
    let mut stack: Vec<Node> = Vec::new();
    let mut roots: Vec<Node> = Vec::new();

    fn attach(stack: &mut Vec<Node>, roots: &mut Vec<Node>, node: Node) {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        } else {
            roots.push(node);
        }
    }

    for (level, node, _spans) in flat {
        while let Some(top) = stack.last() {
            if top.level >= level {
                let done = stack.pop().unwrap();
                attach(&mut stack, &mut roots, done);
            } else {
                break;
            }
        }
        stack.push(node);
    }
    while let Some(done) = stack.pop() {
        attach(&mut stack, &mut roots, done);
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
#+TITLE: Tasks
#+TODO: TODO NEXT | DONE

* TODO Build Orgion
DEADLINE: <2026-10-01 Thu>
:PROPERTIES:
:PROJECT: Orgion
:STATUS: Development
:END:

Some notes about building it.

* NEXT Implement parser

* DONE Create repository
";

    #[test]
    fn parses_preamble() {
        let pd = parse(SAMPLE);
        assert_eq!(pd.doc.preamble.get("TITLE"), Some("Tasks"));
        assert_eq!(pd.doc.todo_keywords.0.len(), 3);
    }

    #[test]
    fn parses_three_top_level_nodes() {
        let pd = parse(SAMPLE);
        assert_eq!(pd.doc.nodes.len(), 3);
        assert_eq!(pd.doc.nodes[0].title, "Build Orgion");
        assert_eq!(pd.doc.nodes[0].todo_state.as_deref(), Some("TODO"));
        assert_eq!(pd.doc.nodes[1].todo_state.as_deref(), Some("NEXT"));
        assert_eq!(pd.doc.nodes[2].todo_state.as_deref(), Some("DONE"));
    }

    #[test]
    fn parses_deadline_and_properties() {
        let pd = parse(SAMPLE);
        let n = &pd.doc.nodes[0];
        assert!(n.planning.deadline.is_some());
        assert_eq!(n.property("PROJECT"), Some("Orgion"));
        assert_eq!(n.property("STATUS"), Some("Development"));
    }

    #[test]
    fn body_excludes_drawer_and_planning() {
        let pd = parse(SAMPLE);
        let n = &pd.doc.nodes[0];
        assert!(!n.body.contains("PROPERTIES"));
        assert!(!n.body.contains("DEADLINE"));
        assert!(n.body.contains("Some notes about building it."));
    }

    #[test]
    fn spans_align_with_flattened_iter_order() {
        let pd = parse(SAMPLE);
        let flattened: Vec<&Node> = pd.doc.iter().collect();
        assert_eq!(flattened.len(), pd.spans.len());
        for (node, span) in flattened.iter().zip(pd.spans.iter()) {
            let heading_text = span.heading_line.slice(SAMPLE);
            assert!(heading_text.contains(&node.title));
        }
    }

    #[test]
    fn nested_headings_build_correct_tree() {
        let src = "\
#+TODO: TODO NEXT | DONE

* Project A
** TODO Task 1
** TODO Task 2
* Project B
** NEXT Task 3
";
        let pd = parse(src);
        assert_eq!(pd.doc.nodes.len(), 2);
        assert_eq!(pd.doc.nodes[0].title, "Project A");
        assert_eq!(pd.doc.nodes[0].children.len(), 2);
        assert_eq!(pd.doc.nodes[0].children[0].title, "Task 1");
        assert_eq!(pd.doc.nodes[1].children.len(), 1);
        assert_eq!(pd.doc.nodes[1].children[0].title, "Task 3");
    }

    #[test]
    fn whole_span_covers_children() {
        let src = "\
* Project A
** TODO Task 1
** TODO Task 2
* Project B
";
        let pd = parse(src);
        // Node 0 = "Project A" (spans[0]) should cover through the end of
        // Task 2, i.e. everything up to "* Project B".
        let project_a_whole = pd.spans[0].whole.slice(src);
        assert!(project_a_whole.contains("Task 1"));
        assert!(project_a_whole.contains("Task 2"));
        assert!(!project_a_whole.contains("Project B"));
    }

    #[test]
    fn extracts_links_from_body() {
        let src = "* Paper\nSee [[id:abc-123][related]] for context.\n";
        let pd = parse(src);
        assert_eq!(pd.doc.nodes[0].links.len(), 1);
        assert_eq!(pd.doc.nodes[0].links[0].target_raw, "abc-123");
    }

    #[test]
    fn handles_file_with_no_trailing_newline() {
        let src = "* TODO Last line no newline";
        let pd = parse(src);
        assert_eq!(pd.doc.nodes.len(), 1);
        assert_eq!(pd.doc.nodes[0].title, "Last line no newline");
    }

    #[test]
    fn handles_empty_file() {
        let pd = parse("");
        assert!(pd.doc.nodes.is_empty());
    }

    #[test]
    fn org_id_populated_from_id_property() {
        let src = "\
* TODO Write HRI Paper
:PROPERTIES:
:ID:       3a42219d-9aac-4935
:END:
";
        let pd = parse(src);
        assert_eq!(pd.doc.nodes[0].org_id.as_deref(), Some("3a42219d-9aac-4935"));
    }
}

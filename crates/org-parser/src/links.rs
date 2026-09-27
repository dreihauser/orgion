//! Extraction of `[[target]]` / `[[target][description]]` links from
//! title or body text.

use org_model::{Link, LinkKind};

pub fn extract_links(text: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some(end) = text[i..].find("]]") {
                let inner = &text[i + 2..i + end];
                let (raw_target, description) = match inner.split_once("][") {
                    Some((t, d)) => (t, Some(d.to_string())),
                    None => (inner, None),
                };
                let (kind, stripped) = LinkKind::classify(raw_target);
                links.push(Link {
                    target_kind: kind,
                    target_raw: stripped.to_string(),
                    description,
                });
                i += end + 2;
                continue;
            }
        }
        i += 1;
    }
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_id_link() {
        let links = extract_links("See [[id:3a42219d-9aac-4935][the paper]] for details.");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_raw, "3a42219d-9aac-4935");
        assert_eq!(links[0].description.as_deref(), Some("the paper"));
        assert!(matches!(links[0].target_kind, LinkKind::Id));
    }

    #[test]
    fn extracts_multiple_links() {
        let links = extract_links("[[file:foo.org]] and [[https://example.com][site]]");
        assert_eq!(links.len(), 2);
        assert!(matches!(links[0].target_kind, LinkKind::File));
        assert!(matches!(links[1].target_kind, LinkKind::Web));
    }

    #[test]
    fn ignores_plain_brackets() {
        assert!(extract_links("array[0] and [not a link]").is_empty());
    }
}

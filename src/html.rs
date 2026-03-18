use ego_tree::NodeRef;
use scraper::{Html, node::Node};

/// Block-level tags that should introduce a newline in the text output.
const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "br",
    "caption",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "legend",
    "li",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "ul",
];

/// Tags whose subtree should be skipped.
const SKIP_TAGS: &[&str] = &["script", "style", "noscript", "template", "head"];

/// Extract visible plain text from HTML, one logical line per block element.
pub fn extract_text(html: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let mut buf = String::new();
    walk_node(document.tree.root(), &mut buf, false);
    lines_from_buf(&buf)
}

fn walk_node(node: NodeRef<Node>, buf: &mut String, in_skip: bool) {
    match node.value() {
        Node::Text(text) => {
            if !in_skip {
                buf.push_str(text);
            }
        }
        Node::Element(el) => {
            let tag = el.name().to_ascii_lowercase();
            let skip = in_skip || SKIP_TAGS.contains(&tag.as_str());
            let is_block = BLOCK_TAGS.contains(&tag.as_str());

            if is_block && !skip {
                ensure_newline(buf);
            }
            for child in node.children() {
                walk_node(child, buf, skip);
            }
            if is_block && !skip {
                ensure_newline(buf);
            }
        }
        _ => {
            for child in node.children() {
                walk_node(child, buf, in_skip);
            }
        }
    }
}

fn ensure_newline(buf: &mut String) {
    if !buf.ends_with('\n') && !buf.is_empty() {
        buf.push('\n');
    }
}

fn lines_from_buf(raw: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut prev_blank = false;

    for line in raw.split('\n') {
        // Normalize whitespace within a line
        let trimmed = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.is_empty() {
            if !prev_blank && !lines.is_empty() {
                lines.push(String::new());
            }
            prev_blank = true;
        } else {
            lines.push(trimmed);
            prev_blank = false;
        }
    }

    // Remove trailing blank line
    while lines.last().map(|l: &String| l.is_empty()).unwrap_or(false) {
        lines.pop();
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_extraction() {
        let html = "<html><body><p>Hello, <b>world</b>!</p></body></html>";
        let lines = extract_text(html);
        assert!(lines.iter().any(|l| l.contains("Hello")));
        assert!(lines.iter().any(|l| l.contains("world")));
    }

    #[test]
    fn test_skips_scripts() {
        let html =
            "<html><head><script>var x = 1;</script></head><body><p>visible</p></body></html>";
        let lines = extract_text(html);
        assert!(lines.iter().all(|l| !l.contains("var x")));
        assert!(lines.iter().any(|l| l.contains("visible")));
    }

    #[test]
    fn test_block_elements_become_lines() {
        let html = "<div>First</div><div>Second</div><div>Third</div>";
        let lines = extract_text(html);
        let non_blank: Vec<_> = lines.iter().filter(|l| !l.is_empty()).collect();
        assert_eq!(non_blank.len(), 3);
    }
}

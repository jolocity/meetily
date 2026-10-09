//! HTML rendering. Styles are inline so tables keep their borders when pasted into
//! Outlook, Word, and other rich text editors that ignore `<style>` blocks.

use super::model::{Block, Span};

const TABLE_STYLE: &str = "border-collapse:collapse;margin:8px 0";
const CELL_STYLE: &str = "border:1px solid #999999;padding:4px 8px;vertical-align:top;text-align:left";
const HEADER_CELL_STYLE: &str =
    "border:1px solid #999999;padding:4px 8px;vertical-align:top;text-align:left;background-color:#f2f2f2";
const CODE_STYLE: &str = "font-family:Consolas,'Courier New',monospace";
const PRE_STYLE: &str =
    "font-family:Consolas,'Courier New',monospace;background-color:#f5f5f5;padding:8px;white-space:pre-wrap";
const QUOTE_STYLE: &str = "border-left:3px solid #cccccc;margin:8px 0;padding-left:12px;color:#555555";
const DOCUMENT_CSS: &str = "body{font-family:Calibri,'Segoe UI',Arial,sans-serif;font-size:11pt;line-height:1.5;max-width:900px;margin:2rem auto;padding:0 1rem;color:#1f1f1f}";

/// Body content only, suitable for the clipboard.
pub fn render_fragment(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        write_block(&mut out, block);
    }
    out
}

/// Standalone HTML file.
pub fn render_document(title: &str, blocks: &[Block]) -> String {
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n<style>{}</style>\n</head>\n<body>\n{}</body>\n</html>\n",
        escape(title),
        DOCUMENT_CSS,
        render_fragment(blocks)
    )
}

fn write_block(out: &mut String, block: &Block) {
    match block {
        Block::Heading { level, spans } => {
            out.push_str(&format!("<h{level}>{}</h{level}>\n", render_spans(spans)));
        }
        Block::Paragraph(spans) => out.push_str(&format!("<p>{}</p>\n", render_spans(spans))),
        Block::List { start, items } => {
            match start {
                Some(1) => out.push_str("<ol>\n"),
                Some(first) => out.push_str(&format!("<ol start=\"{first}\">\n")),
                None => out.push_str("<ul>\n"),
            }
            for item in items {
                out.push_str("<li>");
                write_list_item(out, item);
                out.push_str("</li>\n");
            }
            out.push_str(if start.is_some() { "</ol>\n" } else { "</ul>\n" });
        }
        Block::Table { header, rows } => {
            out.push_str(&format!("<table style=\"{TABLE_STYLE}\">\n"));
            if !header.is_empty() {
                out.push_str("<thead><tr>");
                for cell in header {
                    out.push_str(&format!("<th style=\"{HEADER_CELL_STYLE}\">{}</th>", render_spans(cell)));
                }
                out.push_str("</tr></thead>\n");
            }
            out.push_str("<tbody>\n");
            for row in rows {
                out.push_str("<tr>");
                for cell in row {
                    out.push_str(&format!("<td style=\"{CELL_STYLE}\">{}</td>", render_spans(cell)));
                }
                out.push_str("</tr>\n");
            }
            out.push_str("</tbody>\n</table>\n");
        }
        Block::Code(code) => out.push_str(&format!("<pre style=\"{PRE_STYLE}\">{}</pre>\n", escape(code))),
        Block::Quote(blocks) => {
            out.push_str(&format!("<blockquote style=\"{QUOTE_STYLE}\">\n"));
            for inner in blocks {
                write_block(out, inner);
            }
            out.push_str("</blockquote>\n");
        }
        Block::Rule => out.push_str("<hr>\n"),
    }
}

/// A leading paragraph is written inline so list items do not get paragraph margins.
fn write_list_item(out: &mut String, item: &[Block]) {
    let mut blocks = item.iter();
    if let Some(Block::Paragraph(spans)) = item.first() {
        out.push_str(&render_spans(spans));
        blocks.next();
    }
    for block in blocks {
        write_block(out, block);
    }
}

fn render_spans(spans: &[Span]) -> String {
    spans.iter().map(render_span).collect()
}

fn render_span(span: &Span) -> String {
    let mut html = escape(&span.text).replace('\n', "<br>");
    let style = &span.style;
    if style.code {
        html = format!("<code style=\"{CODE_STYLE}\">{html}</code>");
    }
    if style.bold {
        html = format!("<strong>{html}</strong>");
    }
    if style.italic {
        html = format!("<em>{html}</em>");
    }
    if style.strike {
        html = format!("<s>{html}</s>");
    }
    if let Some(url) = &style.link {
        html = format!("<a href=\"{}\">{html}</a>", escape(url));
    }
    html
}

pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::super::model::parse;
    use super::*;

    #[test]
    fn renders_tables_with_inline_borders() {
        let html = render_fragment(&parse("| A | B |\n| --- | --- |\n| 1 | 2 |"));
        assert!(html.contains("<table style=\"border-collapse:collapse"));
        assert!(html.contains("<th style=\"border:1px solid #999999"));
        assert!(html.contains(">2</td>"));
    }

    #[test]
    fn escapes_text_and_writes_tight_list_items_inline() {
        let html = render_fragment(&parse("- a < b & **c**"));
        assert_eq!(html, "<ul>\n<li>a &lt; b &amp; <strong>c</strong></li>\n</ul>\n");
    }

    #[test]
    fn document_has_title_and_charset() {
        let html = render_document("Q3 \"Plan\"", &parse("Hi"));
        assert!(html.contains("<meta charset=\"utf-8\">"));
        assert!(html.contains("<title>Q3 &quot;Plan&quot;</title>"));
    }
}

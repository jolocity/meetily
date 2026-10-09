//! Plain text rendering for Notepad and other editors without formatting.

use super::model::{spans_text, Block, Span};

const RULE_WIDTH: usize = 40;
const CODE_INDENT: &str = "    ";
const BULLET_MARKER: &str = "- ";
const COLUMN_SEPARATOR: &str = " | ";

pub fn render(blocks: &[Block]) -> String {
    let mut text = render_blocks(blocks, "\n\n");
    text.push('\n');
    text
}

fn render_blocks(blocks: &[Block], separator: &str) -> String {
    blocks.iter().map(render_block).collect::<Vec<_>>().join(separator)
}

fn render_block(block: &Block) -> String {
    match block {
        Block::Heading { level, spans } => {
            let text = inline_text(spans);
            match level {
                1 => underline(&text, '='),
                2 => underline(&text, '-'),
                _ => text,
            }
        }
        Block::Paragraph(spans) => inline_text(spans),
        Block::List { start, items } => render_list(*start, items),
        Block::Table { header, rows } => render_table(header, rows),
        Block::Code(code) => prefix_lines(code, CODE_INDENT, CODE_INDENT),
        Block::Quote(blocks) => prefix_lines(&render_blocks(blocks, "\n\n"), "> ", "> "),
        Block::Rule => "-".repeat(RULE_WIDTH),
    }
}

/// Link targets are appended in parentheses so they survive without formatting.
fn inline_text(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|span| match &span.style.link {
            Some(url) if url != &span.text => format!("{} ({})", span.text, url),
            _ => span.text.clone(),
        })
        .collect()
}

fn underline(text: &str, marker: char) -> String {
    let width = text.lines().map(display_width).max().unwrap_or(0);
    format!("{}\n{}", text, marker.to_string().repeat(width))
}

fn render_list(start: Option<u64>, items: &[Vec<Block>]) -> String {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = match start {
                Some(first) => format!("{}. ", first + index as u64),
                None => BULLET_MARKER.to_string(),
            };
            let continuation = " ".repeat(marker.len());
            prefix_lines(&render_blocks(item, "\n"), &marker, &continuation)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_table(header: &[Vec<Span>], rows: &[Vec<Vec<Span>>]) -> String {
    let cell_text = |cell: &Vec<Span>| spans_text(cell).replace('\n', " ");
    let header: Vec<String> = header.iter().map(cell_text).collect();
    let rows: Vec<Vec<String>> = rows.iter().map(|row| row.iter().map(cell_text).collect()).collect();

    let columns = rows.iter().map(Vec::len).chain(std::iter::once(header.len())).max().unwrap_or(0);
    let mut widths = vec![0; columns];
    for row in std::iter::once(&header).chain(rows.iter()) {
        for (column, cell) in row.iter().enumerate() {
            widths[column] = widths[column].max(display_width(cell));
        }
    }

    let format_row = |row: &[String]| {
        (0..columns)
            .map(|column| {
                let cell = row.get(column).map(String::as_str).unwrap_or("");
                format!("{}{}", cell, " ".repeat(widths[column] - display_width(cell)))
            })
            .collect::<Vec<_>>()
            .join(COLUMN_SEPARATOR)
            .trim_end()
            .to_string()
    };
    let divider = widths.iter().map(|width| "-".repeat(*width)).collect::<Vec<_>>().join("-+-");

    std::iter::once(format_row(&header))
        .chain(std::iter::once(divider))
        .chain(rows.iter().map(|row| format_row(row)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn prefix_lines(text: &str, first: &str, rest: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            let prefix = if index == 0 { first } else { rest };
            if line.is_empty() {
                prefix.trim_end().to_string()
            } else {
                format!("{}{}", prefix, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn display_width(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::super::model::parse;
    use super::*;

    #[test]
    fn aligns_table_columns() {
        let text = render(&parse("| Owner | Due |\n| --- | --- |\n| Ana | Friday |"));
        assert_eq!(text, "Owner | Due\n------+-------\nAna   | Friday\n");
    }

    #[test]
    fn renders_headings_lists_and_links() {
        let text = render(&parse("# Notes\n\n1. First\n   - Child\n2. See [docs](https://example.com)"));
        assert_eq!(text, "Notes\n=====\n\n1. First\n   - Child\n2. See docs (https://example.com)\n");
    }

    #[test]
    fn strips_inline_formatting() {
        assert_eq!(render(&parse("**Bold** and ~~gone~~")), "Bold and gone\n");
    }
}

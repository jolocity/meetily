//! Rich Text Format rendering for WordPad and older editors.

use super::model::{Block, Span};

const BODY_FONT_SIZE: u32 = 22;
const CODE_FONT_SIZE: u32 = 20;
const HEADING_FONT_SIZES: [u32; 4] = [32, 28, 24, 22];
const INDENT_STEP_TWIPS: usize = 720;
const HANGING_INDENT_TWIPS: usize = 360;
const TABLE_WIDTH_TWIPS: usize = 9360;
const HEADER_SHADING_COLOR: u8 = 2;
const LINK_COLOR: u8 = 1;

const HEADER: &str = "{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1\n{\\fonttbl{\\f0\\fswiss Calibri;}{\\f1\\fmodern Consolas;}}\n{\\colortbl ;\\red5\\green99\\blue193;\\red242\\green242\\blue242;}\n";

pub fn render(blocks: &[Block]) -> String {
    let mut out = String::from(HEADER);
    write_blocks(&mut out, blocks, Context::default());
    out.push_str("}\n");
    out
}

#[derive(Clone, Copy, Default)]
struct Context {
    list_depth: usize,
    indent: usize,
    quote: bool,
}

fn write_blocks(out: &mut String, blocks: &[Block], ctx: Context) {
    for block in blocks {
        write_block(out, block, ctx);
    }
}

fn write_block(out: &mut String, block: &Block, ctx: Context) {
    match block {
        Block::Heading { level, spans } => {
            let size = HEADING_FONT_SIZES[(*level as usize).clamp(1, HEADING_FONT_SIZES.len()) - 1];
            out.push_str(&format!(
                "{{\\pard\\sb240\\sa120\\keepn{}\\b\\fs{size} {}\\par}}\n",
                indent_control(ctx.indent),
                runs(spans)
            ));
        }
        Block::Paragraph(spans) => out.push_str(&format!(
            "{{\\pard\\sa120{}{}\\fs{BODY_FONT_SIZE} {}\\par}}\n",
            indent_control(ctx.indent + usize::from(ctx.quote)),
            if ctx.quote { "\\i" } else { "" },
            runs(spans)
        )),
        Block::List { start, items } => {
            let left = INDENT_STEP_TWIPS * (ctx.list_depth + 1);
            let nested = Context { list_depth: ctx.list_depth + 1, indent: ctx.list_depth + 1, ..ctx };
            for (index, item) in items.iter().enumerate() {
                let marker = match start {
                    Some(first) => format!("{}.", first + index as u64),
                    None => "\\bullet".to_string(),
                };
                let (first_text, rest) = match item.split_first() {
                    Some((Block::Paragraph(spans), rest)) => (runs(spans), rest),
                    _ => (String::new(), item.as_slice()),
                };
                out.push_str(&format!(
                    "{{\\pard\\sa60\\fi-{HANGING_INDENT_TWIPS}\\li{left}\\fs{BODY_FONT_SIZE} {marker}\\tab {first_text}\\par}}\n"
                ));
                write_blocks(out, rest, nested);
            }
        }
        Block::Table { header, rows } => write_table(out, header, rows),
        Block::Code(code) => {
            for line in code.split('\n') {
                out.push_str(&format!(
                    "{{\\pard\\sa0{}\\f1\\fs{CODE_FONT_SIZE} {}\\par}}\n",
                    indent_control(ctx.indent + 1),
                    escape(line)
                ));
            }
        }
        Block::Quote(blocks) => write_blocks(out, blocks, Context { quote: true, ..ctx }),
        Block::Rule => out.push_str("{\\pard\\brdrb\\brdrs\\brdrw10\\brsp20 \\par}\n"),
    }
}

fn write_table(out: &mut String, header: &[Vec<Span>], rows: &[Vec<Vec<Span>>]) {
    let columns = rows.iter().map(Vec::len).chain(std::iter::once(header.len())).max().unwrap_or(0);
    if columns == 0 {
        return;
    }
    let column_width = TABLE_WIDTH_TWIPS / columns;

    let mut write_row = |cells: &[Vec<Span>], is_header: bool| {
        out.push_str("\\trowd\\trgaph108\\trleft0");
        if is_header {
            out.push_str("\\trhdr");
        }
        for column in 0..columns {
            out.push_str("\\clbrdrt\\brdrs\\brdrw10\\clbrdrl\\brdrs\\brdrw10\\clbrdrb\\brdrs\\brdrw10\\clbrdrr\\brdrs\\brdrw10");
            if is_header {
                out.push_str(&format!("\\clcbpat{HEADER_SHADING_COLOR}"));
            }
            out.push_str(&format!("\\cellx{}", column_width * (column + 1)));
        }
        out.push('\n');
        for column in 0..columns {
            let text = cells.get(column).map(|cell| runs(cell)).unwrap_or_default();
            let bold = if is_header { "\\b" } else { "" };
            out.push_str(&format!("\\pard\\intbl\\fs{BODY_FONT_SIZE}{{{bold} {text}}}\\cell\n"));
        }
        out.push_str("\\row\n");
    };

    if !header.is_empty() {
        write_row(header, true);
    }
    for row in rows {
        write_row(row, false);
    }
    out.push_str("\\pard\\sa120\\par\n");
}

fn indent_control(steps: usize) -> String {
    if steps == 0 {
        String::new()
    } else {
        format!("\\li{}", INDENT_STEP_TWIPS * steps)
    }
}

fn runs(spans: &[Span]) -> String {
    spans.iter().map(run).collect()
}

fn run(span: &Span) -> String {
    let style = &span.style;
    let mut controls = String::new();
    if style.bold {
        controls.push_str("\\b");
    }
    if style.italic {
        controls.push_str("\\i");
    }
    if style.strike {
        controls.push_str("\\strike");
    }
    if style.code {
        controls.push_str(&format!("\\f1\\fs{CODE_FONT_SIZE}"));
    }
    let text = escape(&span.text);
    let group = if controls.is_empty() { text } else { format!("{{{controls} {text}}}") };
    match &style.link {
        Some(url) => format!(
            "{{\\field{{\\*\\fldinst{{HYPERLINK \"{}\"}}}}{{\\fldrslt{{\\ul\\cf{LINK_COLOR} {group}}}}}}}",
            escape(url)
        ),
        None => group,
    }
}

/// Escapes RTF control characters and writes non-ASCII text as `\uN?` (UTF-16 code units).
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' | '{' | '}' => {
                escaped.push('\\');
                escaped.push(c);
            }
            '\n' => escaped.push_str("\\line "),
            '\t' => escaped.push_str("\\tab "),
            c if c.is_ascii() && !c.is_ascii_control() => escaped.push(c),
            c if c.is_ascii_control() => {}
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    escaped.push_str(&format!("\\u{}?", *unit as i16));
                }
            }
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::super::model::parse;
    use super::*;

    #[test]
    fn escapes_control_characters_and_unicode() {
        assert_eq!(escape("a{b}\\c"), "a\\{b\\}\\\\c");
        assert_eq!(escape("caf\u{e9}"), "caf\\u233?");
        assert_eq!(escape("\u{1F600}"), "\\u-10179?\\u-8704?");
    }

    #[test]
    fn renders_headings_lists_and_tables() {
        let rtf = render(&parse("# Title\n\n- one\n\n| A | B |\n| --- | --- |\n| 1 | 2 |"));
        assert!(rtf.starts_with("{\\rtf1"));
        assert!(rtf.trim_end().ends_with('}'));
        assert!(rtf.contains("\\b\\fs32 Title\\par"));
        assert!(rtf.contains("\\bullet\\tab one\\par"));
        assert_eq!(rtf.matches("\\cell\n").count(), 4);
        assert_eq!(rtf.matches("\\row").count(), 2);
    }

    #[test]
    fn renders_hyperlinks_as_fields() {
        let rtf = render(&parse("[docs](https://example.com)"));
        assert!(rtf.contains("{\\field{\\*\\fldinst{HYPERLINK \"https://example.com\"}}{\\fldrslt{\\ul\\cf1 docs}}}"));
    }
}

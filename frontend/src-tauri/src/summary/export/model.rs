//! Markdown parsed into a small document model shared by every export format.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub link: Option<String>,
}

/// A run of text with a single style. Line breaks inside a paragraph are kept as `\n`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
}

pub type Spans = Vec<Span>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading { level: u8, spans: Spans },
    Paragraph(Spans),
    /// `start` is `Some` for ordered lists. Each item is a list of blocks.
    List { start: Option<u64>, items: Vec<Vec<Block>> },
    Table { header: Vec<Spans>, rows: Vec<Vec<Spans>> },
    Code(String),
    Quote(Vec<Block>),
    Rule,
}

pub fn spans_text(spans: &[Span]) -> String {
    spans.iter().map(|span| span.text.as_str()).collect()
}

/// Parse CommonMark plus GitHub tables, strikethrough, and task lists.
pub fn parse(markdown: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut builder = Builder::new();

    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Start(tag) => builder.start(tag),
            Event::End(tag) => builder.end(tag),
            Event::Text(text) => builder.push_text(&text, false),
            Event::Code(text) => builder.push_text(&text, true),
            Event::InlineHtml(html) | Event::Html(html) => {
                if is_line_break_tag(&html) {
                    builder.push_text("\n", false);
                }
            }
            Event::SoftBreak => builder.push_text(" ", false),
            Event::HardBreak => builder.push_text("\n", false),
            Event::Rule => {
                builder.flush_inline();
                builder.push_block(Block::Rule);
            }
            Event::TaskListMarker(done) => builder.push_text(if done { "[x] " } else { "[ ] " }, false),
            _ => {}
        }
    }

    builder.finish()
}

fn is_line_break_tag(html: &str) -> bool {
    let tag: String = html.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
    matches!(tag.as_str(), "<br>" | "<br/>")
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

enum Container {
    Root(Vec<Block>),
    Quote(Vec<Block>),
    List { start: Option<u64>, items: Vec<Vec<Block>> },
    Item(Vec<Block>),
}

enum InlineKind {
    Paragraph,
    Heading(u8),
    Cell,
}

#[derive(Default)]
struct TableState {
    header: Vec<Spans>,
    rows: Vec<Vec<Spans>>,
    row: Vec<Spans>,
}

struct Builder {
    containers: Vec<Container>,
    inline: Option<(InlineKind, Spans)>,
    styles: Vec<SpanStyle>,
    table: Option<TableState>,
    code: Option<String>,
}

impl Builder {
    fn new() -> Self {
        Self {
            containers: vec![Container::Root(Vec::new())],
            inline: None,
            styles: vec![SpanStyle::default()],
            table: None,
            code: None,
        }
    }

    fn current_style(&self) -> SpanStyle {
        self.styles.last().cloned().unwrap_or_default()
    }

    fn push_style(&mut self, apply: impl FnOnce(&mut SpanStyle)) {
        let mut style = self.current_style();
        apply(&mut style);
        self.styles.push(style);
    }

    fn pop_style(&mut self) {
        if self.styles.len() > 1 {
            self.styles.pop();
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.containers.last_mut() {
            Some(Container::Root(blocks)) | Some(Container::Quote(blocks)) | Some(Container::Item(blocks)) => {
                blocks.push(block)
            }
            Some(Container::List { items, .. }) => items.push(vec![block]),
            None => {}
        }
    }

    fn push_text(&mut self, text: &str, code: bool) {
        if let Some(buffer) = self.code.as_mut() {
            buffer.push_str(text);
            return;
        }

        let mut style = self.current_style();
        style.code |= code;
        let (_, spans) = self.inline.get_or_insert_with(|| (InlineKind::Paragraph, Vec::new()));
        match spans.last_mut() {
            Some(last) if last.style == style => last.text.push_str(text),
            _ => spans.push(Span { text: text.to_string(), style }),
        }
    }

    /// Close the open paragraph, heading, or table cell. Tight list items have no explicit
    /// paragraph, so this also runs before any nested block starts.
    fn flush_inline(&mut self) {
        let Some((kind, spans)) = self.inline.take() else { return };
        match kind {
            InlineKind::Paragraph if !spans.is_empty() => self.push_block(Block::Paragraph(spans)),
            InlineKind::Paragraph => {}
            InlineKind::Heading(level) => self.push_block(Block::Heading { level, spans }),
            InlineKind::Cell => {
                if let Some(table) = self.table.as_mut() {
                    table.row.push(spans);
                }
            }
        }
    }

    fn pop_container(&mut self) -> Option<Container> {
        if self.containers.len() > 1 {
            self.containers.pop()
        } else {
            None
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.flush_inline();
                self.inline = Some((InlineKind::Paragraph, Vec::new()));
            }
            Tag::Heading { level, .. } => {
                self.flush_inline();
                self.inline = Some((InlineKind::Heading(heading_level(level)), Vec::new()));
            }
            Tag::BlockQuote(_) => {
                self.flush_inline();
                self.containers.push(Container::Quote(Vec::new()));
            }
            Tag::CodeBlock(_) => {
                self.flush_inline();
                self.code = Some(String::new());
            }
            Tag::List(start) => {
                self.flush_inline();
                self.containers.push(Container::List { start, items: Vec::new() });
            }
            Tag::Item => {
                self.flush_inline();
                self.containers.push(Container::Item(Vec::new()));
            }
            Tag::Table(_) => {
                self.flush_inline();
                self.table = Some(TableState::default());
            }
            Tag::TableHead | Tag::TableRow => {
                if let Some(table) = self.table.as_mut() {
                    table.row.clear();
                }
            }
            Tag::TableCell => self.inline = Some((InlineKind::Cell, Vec::new())),
            Tag::Emphasis => self.push_style(|style| style.italic = true),
            Tag::Strong => self.push_style(|style| style.bold = true),
            Tag::Strikethrough => self.push_style(|style| style.strike = true),
            Tag::Link { dest_url, .. } => {
                let url = dest_url.to_string();
                self.push_style(|style| style.link = Some(url));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::TableCell | TagEnd::HtmlBlock => self.flush_inline(),
            TagEnd::BlockQuote(_) => {
                self.flush_inline();
                if let Some(Container::Quote(blocks)) = self.pop_container() {
                    self.push_block(Block::Quote(blocks));
                }
            }
            TagEnd::CodeBlock => {
                if let Some(code) = self.code.take() {
                    self.push_block(Block::Code(code.trim_end_matches('\n').to_string()));
                }
            }
            TagEnd::List(_) => {
                self.flush_inline();
                if let Some(Container::List { start, items }) = self.pop_container() {
                    self.push_block(Block::List { start, items });
                }
            }
            TagEnd::Item => {
                self.flush_inline();
                if let Some(Container::Item(blocks)) = self.pop_container() {
                    if let Some(Container::List { items, .. }) = self.containers.last_mut() {
                        items.push(blocks);
                    }
                }
            }
            TagEnd::TableHead => {
                if let Some(table) = self.table.as_mut() {
                    table.header = std::mem::take(&mut table.row);
                }
            }
            TagEnd::TableRow => {
                if let Some(table) = self.table.as_mut() {
                    let row = std::mem::take(&mut table.row);
                    table.rows.push(row);
                }
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    self.push_block(Block::Table { header: table.header, rows: table.rows });
                }
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => self.pop_style(),
            _ => {}
        }
    }

    fn finish(mut self) -> Vec<Block> {
        self.flush_inline();
        match self.containers.into_iter().next() {
            Some(Container::Root(blocks)) => blocks,
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Span {
        Span { text: text.to_string(), style: SpanStyle::default() }
    }

    #[test]
    fn parses_headings_paragraphs_and_inline_styles() {
        let blocks = parse("# Title\n\nSome **bold** and *italic* `code`.");
        assert_eq!(blocks[0], Block::Heading { level: 1, spans: vec![plain("Title")] });
        let Block::Paragraph(spans) = &blocks[1] else { panic!("expected paragraph") };
        assert!(spans.iter().any(|s| s.text == "bold" && s.style.bold));
        assert!(spans.iter().any(|s| s.text == "italic" && s.style.italic));
        assert!(spans.iter().any(|s| s.text == "code" && s.style.code));
    }

    #[test]
    fn parses_tight_and_nested_lists() {
        let blocks = parse("1. First\n   - Child\n2. Second");
        let Block::List { start, items } = &blocks[0] else { panic!("expected list") };
        assert_eq!(*start, Some(1));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0][0], Block::Paragraph(vec![plain("First")]));
        assert!(matches!(&items[0][1], Block::List { start: None, items } if items.len() == 1));
    }

    #[test]
    fn parses_tables() {
        let blocks = parse("| Owner | Due |\n| --- | --- |\n| Ana | Friday |\n| Raj | Monday |");
        let Block::Table { header, rows } = &blocks[0] else { panic!("expected table") };
        assert_eq!(header.iter().map(|c| spans_text(c)).collect::<Vec<_>>(), ["Owner", "Due"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(spans_text(&rows[1][0]), "Raj");
    }

    #[test]
    fn keeps_links_and_line_breaks() {
        let blocks = parse("See [docs](https://example.com)<br>next line");
        let Block::Paragraph(spans) = &blocks[0] else { panic!("expected paragraph") };
        assert!(spans.iter().any(|s| s.style.link.as_deref() == Some("https://example.com")));
        assert!(spans_text(spans).contains("\nnext line"));
    }
}

//! Word (.docx) rendering. Writes a minimal WordprocessingML package with heading,
//! list, table, code, and quote styles so the document is editable in Word.

use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::model::{Block, Span};

const BULLET_NUM_ID: usize = 1;
const FIRST_ORDERED_NUM_ID: usize = 2;
const MAX_HEADING_STYLE: u8 = 4;
const MAX_LIST_LEVEL: usize = 8;
const INDENT_STEP_TWIPS: usize = 720;
const HANGING_INDENT_TWIPS: usize = 360;
const FIRST_HYPERLINK_REL_ID: usize = 10;

const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>"#;

const ROOT_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:eastAsia="Calibri" w:cs="Calibri"/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120" w:line="264" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="360" w:after="120"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:color w:val="1F3864"/><w:sz w:val="32"/><w:szCs w:val="32"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="120"/><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:b/><w:color w:val="2F5496"/><w:sz w:val="26"/><w:szCs w:val="26"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="200" w:after="80"/><w:outlineLvl w:val="2"/></w:pPr><w:rPr><w:b/><w:color w:val="2F5496"/><w:sz w:val="24"/><w:szCs w:val="24"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading4"><w:name w:val="heading 4"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="160" w:after="80"/><w:outlineLvl w:val="3"/></w:pPr><w:rPr><w:b/><w:i/><w:color w:val="2F5496"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="ListParagraph"><w:name w:val="List Paragraph"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="40"/><w:contextualSpacing/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Code"><w:name w:val="Code"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/><w:shd w:val="clear" w:color="auto" w:fill="F5F5F5"/></w:pPr><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas" w:cs="Consolas"/><w:sz w:val="20"/><w:szCs w:val="20"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Quote"><w:name w:val="Quote"/><w:basedOn w:val="Normal"/><w:pPr><w:ind w:left="720"/><w:pBdr><w:left w:val="single" w:sz="12" w:space="8" w:color="CCCCCC"/></w:pBdr></w:pPr><w:rPr><w:i/><w:color w:val="555555"/></w:rPr></w:style><w:style w:type="character" w:styleId="Hyperlink"><w:name w:val="Hyperlink"/><w:rPr><w:color w:val="0563C1"/><w:u w:val="single"/></w:rPr></w:style><w:style w:type="character" w:styleId="InlineCode"><w:name w:val="Inline Code"/><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas" w:cs="Consolas"/><w:sz w:val="20"/></w:rPr></w:style><w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/><w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="999999"/><w:left w:val="single" w:sz="4" w:space="0" w:color="999999"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="999999"/><w:right w:val="single" w:sz="4" w:space="0" w:color="999999"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="999999"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="999999"/></w:tblBorders><w:tblCellMar><w:left w:w="108" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style></w:styles>"#;

const BULLET_GLYPHS: [&str; 3] = ["\u{2022}", "\u{25E6}", "\u{25AA}"];

pub fn render(blocks: &[Block]) -> Result<Vec<u8>, String> {
    let mut writer = DocumentWriter::default();
    writer.write_blocks(blocks, Context::default());

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let parts = [
        ("[Content_Types].xml", CONTENT_TYPES_XML.to_string()),
        ("_rels/.rels", ROOT_RELS_XML.to_string()),
        ("word/_rels/document.xml.rels", writer.relationships_xml()),
        ("word/document.xml", writer.document_xml()),
        ("word/styles.xml", STYLES_XML.to_string()),
        ("word/numbering.xml", writer.numbering_xml()),
    ];
    for (name, content) in parts {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
    }
    let cursor = zip.finish().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

#[derive(Clone, Copy, Default)]
struct Context {
    /// Nesting depth of the enclosing list, used as the numbering level.
    list_depth: usize,
    /// Left indent (in steps) for non-numbered paragraphs inside list items.
    indent: usize,
    quote: bool,
}

#[derive(Default)]
struct DocumentWriter {
    body: String,
    hyperlinks: Vec<String>,
    /// (start number, list level) for each ordered list, in order of appearance.
    ordered_lists: Vec<(u64, usize)>,
}

impl DocumentWriter {
    fn write_blocks(&mut self, blocks: &[Block], ctx: Context) {
        for block in blocks {
            self.write_block(block, ctx);
        }
    }

    fn write_block(&mut self, block: &Block, ctx: Context) {
        match block {
            Block::Heading { level, spans } => {
                let style = format!("Heading{}", (*level).min(MAX_HEADING_STYLE));
                self.paragraph(Some(&style), None, ctx, spans);
            }
            Block::Paragraph(spans) => self.paragraph(None, None, ctx, spans),
            Block::List { start, items } => self.write_list(*start, items, ctx),
            Block::Table { header, rows } => self.write_table(header, rows),
            Block::Code(code) => {
                let pr = paragraph_properties(Some("Code"), None, ctx.indent);
                for line in code.split('\n') {
                    self.body.push_str(&format!("<w:p>{pr}{}</w:p>", text_run(line, "")));
                }
                self.body.push_str("<w:p/>");
            }
            Block::Quote(blocks) => self.write_blocks(blocks, Context { quote: true, ..ctx }),
            Block::Rule => self.body.push_str(
                r#"<w:p><w:pPr><w:pBdr><w:bottom w:val="single" w:sz="6" w:space="1" w:color="999999"/></w:pBdr></w:pPr></w:p>"#,
            ),
        }
    }

    fn write_list(&mut self, start: Option<u64>, items: &[Vec<Block>], ctx: Context) {
        let level = ctx.list_depth.min(MAX_LIST_LEVEL);
        let num_id = match start {
            Some(first) => {
                self.ordered_lists.push((first, level));
                FIRST_ORDERED_NUM_ID + self.ordered_lists.len() - 1
            }
            None => BULLET_NUM_ID,
        };
        let nested = Context { list_depth: ctx.list_depth + 1, indent: ctx.list_depth + 1, ..ctx };

        for item in items {
            let mut blocks = item.iter();
            match item.first() {
                Some(Block::Paragraph(spans)) => {
                    blocks.next();
                    self.paragraph(Some("ListParagraph"), Some((num_id, level)), ctx, spans);
                }
                _ => self.paragraph(Some("ListParagraph"), Some((num_id, level)), ctx, &[]),
            }
            for block in blocks {
                self.write_block(block, nested);
            }
        }
    }

    fn write_table(&mut self, header: &[Vec<Span>], rows: &[Vec<Vec<Span>>]) {
        let columns = rows.iter().map(Vec::len).chain(std::iter::once(header.len())).max().unwrap_or(0);
        if columns == 0 {
            return;
        }

        self.body.push_str(r#"<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/><w:tblW w:w="5000" w:type="pct"/></w:tblPr><w:tblGrid>"#);
        for _ in 0..columns {
            self.body.push_str("<w:gridCol/>");
        }
        self.body.push_str("</w:tblGrid>");

        if !header.is_empty() {
            self.table_row(header, columns, true);
        }
        for row in rows {
            self.table_row(row, columns, false);
        }
        // Word needs a paragraph between consecutive tables and after a trailing table.
        self.body.push_str("</w:tbl><w:p/>");
    }

    fn table_row(&mut self, cells: &[Vec<Span>], columns: usize, is_header: bool) {
        self.body.push_str(if is_header { "<w:tr><w:trPr><w:tblHeader/></w:trPr>" } else { "<w:tr>" });
        for column in 0..columns {
            let shading = if is_header { r#"<w:shd w:val="clear" w:color="auto" w:fill="F2F2F2"/>"# } else { "" };
            self.body.push_str(&format!(r#"<w:tc><w:tcPr><w:tcW w:w="0" w:type="auto"/>{shading}</w:tcPr><w:p><w:pPr><w:spacing w:after="0"/></w:pPr>"#));
            if let Some(cell) = cells.get(column) {
                let runs = self.runs(cell, is_header);
                self.body.push_str(&runs);
            }
            self.body.push_str("</w:p></w:tc>");
        }
        self.body.push_str("</w:tr>");
    }

    fn paragraph(&mut self, style: Option<&str>, numbering: Option<(usize, usize)>, ctx: Context, spans: &[Span]) {
        let style = style.or(if ctx.quote { Some("Quote") } else { None });
        let indent = if numbering.is_some() { 0 } else { ctx.indent };
        let pr = paragraph_properties(style, numbering, indent);
        let runs = self.runs(spans, false);
        self.body.push_str(&format!("<w:p>{pr}{runs}</w:p>"));
    }

    fn runs(&mut self, spans: &[Span], force_bold: bool) -> String {
        let mut xml = String::new();
        for span in spans {
            let style = &span.style;
            let mut props = String::new();
            if style.link.is_some() {
                props.push_str(r#"<w:rStyle w:val="Hyperlink"/>"#);
            } else if style.code {
                props.push_str(r#"<w:rStyle w:val="InlineCode"/>"#);
            }
            if style.bold || force_bold {
                props.push_str("<w:b/>");
            }
            if style.italic {
                props.push_str("<w:i/>");
            }
            if style.strike {
                props.push_str("<w:strike/>");
            }
            let run = text_run(&span.text, &props);
            match &style.link {
                Some(url) => {
                    self.hyperlinks.push(url.clone());
                    let rel_id = FIRST_HYPERLINK_REL_ID + self.hyperlinks.len() - 1;
                    xml.push_str(&format!(r#"<w:hyperlink r:id="rId{rel_id}">{run}</w:hyperlink>"#));
                }
                None => xml.push_str(&run),
            }
        }
        xml
    }

    fn document_xml(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>{}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>"#,
            self.body
        )
    }

    fn relationships_xml(&self) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>"#,
        );
        for (index, url) in self.hyperlinks.iter().enumerate() {
            xml.push_str(&format!(
                r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="{}" TargetMode="External"/>"#,
                FIRST_HYPERLINK_REL_ID + index,
                escape_xml(url)
            ));
        }
        xml.push_str("</Relationships>");
        xml
    }

    fn numbering_xml(&self) -> String {
        let levels = |ordered: bool| {
            (0..=MAX_LIST_LEVEL)
                .map(|level| {
                    let (format, text) = if ordered {
                        ("decimal", format!("%{}.", level + 1))
                    } else {
                        ("bullet", BULLET_GLYPHS[level % BULLET_GLYPHS.len()].to_string())
                    };
                    format!(
                        r#"<w:lvl w:ilvl="{level}"><w:start w:val="1"/><w:numFmt w:val="{format}"/><w:lvlText w:val="{text}"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="{}" w:hanging="{HANGING_INDENT_TWIPS}"/></w:pPr></w:lvl>"#,
                        INDENT_STEP_TWIPS * (level + 1)
                    )
                })
                .collect::<String>()
        };

        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:multiLevelType w:val="hybridMultilevel"/>{}</w:abstractNum><w:abstractNum w:abstractNumId="1"><w:multiLevelType w:val="hybridMultilevel"/>{}</w:abstractNum><w:num w:numId="{BULLET_NUM_ID}"><w:abstractNumId w:val="0"/></w:num>"#,
            levels(false),
            levels(true)
        );
        for (index, (start, level)) in self.ordered_lists.iter().enumerate() {
            xml.push_str(&format!(
                r#"<w:num w:numId="{}"><w:abstractNumId w:val="1"/><w:lvlOverride w:ilvl="{level}"><w:startOverride w:val="{start}"/></w:lvlOverride></w:num>"#,
                FIRST_ORDERED_NUM_ID + index
            ));
        }
        xml.push_str("</w:numbering>");
        xml
    }
}

fn paragraph_properties(style: Option<&str>, numbering: Option<(usize, usize)>, indent: usize) -> String {
    let mut props = String::new();
    if let Some(style) = style {
        props.push_str(&format!(r#"<w:pStyle w:val="{style}"/>"#));
    }
    if let Some((num_id, level)) = numbering {
        props.push_str(&format!(r#"<w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="{num_id}"/></w:numPr>"#));
    }
    if indent > 0 {
        props.push_str(&format!(r#"<w:ind w:left="{}"/>"#, INDENT_STEP_TWIPS * indent));
    }
    if props.is_empty() {
        String::new()
    } else {
        format!("<w:pPr>{props}</w:pPr>")
    }
}

/// A run with line breaks preserved as `<w:br/>`.
fn text_run(text: &str, props: &str) -> String {
    let props = if props.is_empty() { String::new() } else { format!("<w:rPr>{props}</w:rPr>") };
    let content = text
        .split('\n')
        .map(|line| format!(r#"<w:t xml:space="preserve">{}</w:t>"#, escape_xml(line)))
        .collect::<Vec<_>>()
        .join("<w:br/>");
    format!("<w:r>{props}{content}</w:r>")
}

/// Escapes markup characters and drops control characters that XML 1.0 does not allow.
fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\t' | '\n' | '\r' => escaped.push(c),
            c if (c as u32) < 0x20 => {}
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::super::model::parse;
    use super::*;

    fn read_part(bytes: &[u8], name: &str) -> String {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
        let mut part = archive.by_name(name).expect("part exists");
        let mut content = String::new();
        part.read_to_string(&mut content).unwrap();
        content
    }

    #[test]
    fn writes_a_complete_package() {
        let bytes = render(&parse("# Title\n\nHello")).unwrap();
        let archive = zip::ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        let mut names: Vec<_> = archive.file_names().collect();
        names.sort();
        assert_eq!(
            names,
            ["[Content_Types].xml", "_rels/.rels", "word/_rels/document.xml.rels", "word/document.xml", "word/numbering.xml", "word/styles.xml"]
        );
        let document = read_part(&bytes, "word/document.xml");
        assert!(document.contains(r#"<w:pStyle w:val="Heading1"/>"#));
        assert!(document.contains(">Hello</w:t>"));
    }

    #[test]
    fn writes_tables_lists_and_hyperlinks() {
        let markdown = "| A | B |\n| --- | --- |\n| 1 | 2 |\n\n1. one\n2. two\n\n- [site](https://example.com?a=1&b=2)";
        let bytes = render(&parse(markdown)).unwrap();
        let document = read_part(&bytes, "word/document.xml");
        assert!(document.contains(r#"<w:tblStyle w:val="TableGrid"/>"#));
        assert_eq!(document.matches("<w:tc>").count(), 4);
        assert!(document.contains(r#"<w:numId w:val="2"/>"#));
        assert!(document.contains(r#"<w:numId w:val="1"/>"#));
        let rels = read_part(&bytes, "word/_rels/document.xml.rels");
        assert!(rels.contains(r#"Target="https://example.com?a=1&amp;b=2" TargetMode="External""#));
        let numbering = read_part(&bytes, "word/numbering.xml");
        assert!(numbering.contains(r#"<w:startOverride w:val="1"/>"#));
    }

    #[test]
    fn escapes_xml_and_drops_control_characters() {
        assert_eq!(escape_xml("a<b & \u{0007}c"), "a&lt;b &amp; c");
    }
}

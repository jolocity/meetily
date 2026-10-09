//! Copy and export of generated summaries.
//!
//! The summary markdown is parsed once into a small document model and rendered as
//! plain text, HTML, Word (.docx), or RTF. The frontend uses the plain text and HTML
//! renderings for the clipboard; files are written after a native save dialog.

mod docx;
mod html;
mod model;
mod plain;
mod rtf;

use log::{info, warn};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

const DEFAULT_FILE_NAME: &str = "Meeting Summary";
const INVALID_FILE_NAME_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedSummary {
    /// HTML fragment with inline styles, for rich text clipboard content.
    pub html: String,
    pub plain_text: String,
    /// Standalone HTML document, used for printing to PDF.
    pub html_document: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Docx,
    Md,
    Txt,
    Html,
    Rtf,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            ExportFormat::Docx => "docx",
            ExportFormat::Md => "md",
            ExportFormat::Txt => "txt",
            ExportFormat::Html => "html",
            ExportFormat::Rtf => "rtf",
        }
    }

    fn filter_name(self) -> &'static str {
        match self {
            ExportFormat::Docx => "Word Document",
            ExportFormat::Md => "Markdown",
            ExportFormat::Txt => "Plain Text",
            ExportFormat::Html => "Web Page",
            ExportFormat::Rtf => "Rich Text Format",
        }
    }
}

pub fn render_summary(markdown: &str, title: &str) -> RenderedSummary {
    let blocks = model::parse(markdown);
    RenderedSummary {
        html: html::render_fragment(&blocks),
        plain_text: plain::render(&blocks),
        html_document: html::render_document(title, &blocks),
    }
}

pub fn render_export(format: ExportFormat, markdown: &str, title: &str) -> Result<Vec<u8>, String> {
    let blocks = model::parse(markdown);
    Ok(match format {
        ExportFormat::Md => markdown.as_bytes().to_vec(),
        ExportFormat::Txt => plain::render(&blocks).into_bytes(),
        ExportFormat::Html => html::render_document(title, &blocks).into_bytes(),
        ExportFormat::Rtf => rtf::render(&blocks).into_bytes(),
        ExportFormat::Docx => docx::render(&blocks)?,
    })
}

/// Replaces characters Windows does not allow in file names.
pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if INVALID_FILE_NAME_CHARS.contains(&c) || c.is_control() { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').trim();
    if trimmed.is_empty() {
        DEFAULT_FILE_NAME.to_string()
    } else {
        trimmed.to_string()
    }
}

#[tauri::command]
pub async fn api_render_summary(markdown: String, title: String) -> Result<RenderedSummary, String> {
    Ok(render_summary(&markdown, &title))
}

/// Shows a save dialog and writes the summary in the chosen format.
/// Returns the saved path, or `None` when the dialog is cancelled.
#[tauri::command]
pub async fn api_export_summary<R: Runtime>(
    app: AppHandle<R>,
    markdown: String,
    title: String,
    format: ExportFormat,
    file_name: String,
) -> Result<Option<String>, String> {
    let extension = format.extension();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Save summary as")
        .set_file_name(format!("{}.{}", sanitize_file_name(&file_name), extension))
        .add_filter(format.filter_name(), &[extension])
        .save_file(move |path| {
            let _ = sender.send(path);
        });

    let Some(path) = receiver.await.map_err(|e| e.to_string())? else {
        info!("Summary export cancelled");
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;

    let bytes = render_export(format, &markdown, &title)?;
    std::fs::write(&path, bytes).map_err(|e| {
        warn!("Failed to write summary export to {}: {}", path.display(), e);
        format!("Failed to save file: {}", e)
    })?;

    info!("Summary exported as {} to {}", extension, path.display());
    Ok(Some(path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_file_names() {
        assert_eq!(sanitize_file_name("Q3: Plan / Review?"), "Q3_ Plan _ Review_");
        assert_eq!(sanitize_file_name("  ...  "), DEFAULT_FILE_NAME);
    }

    #[test]
    fn markdown_export_is_unchanged() {
        let markdown = "# Title\n\n| A |\n| - |\n| 1 |\n";
        assert_eq!(render_export(ExportFormat::Md, markdown, "t").unwrap(), markdown.as_bytes());
    }

    #[test]
    fn formats_deserialize_from_lowercase_names() {
        let format: ExportFormat = serde_json::from_str("\"docx\"").unwrap();
        assert_eq!(format, ExportFormat::Docx);
        assert_eq!(format.extension(), "docx");
    }
}

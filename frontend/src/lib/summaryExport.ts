import { invoke } from '@tauri-apps/api/core';

export type CopyFormat = 'rich' | 'plain' | 'markdown' | 'html';
export type ExportFormat = 'docx' | 'pdf' | 'md' | 'txt' | 'html' | 'rtf';

interface RenderedSummary {
  html: string;
  plainText: string;
  htmlDocument: string;
}

const PRINT_CLEANUP_DELAY_MS = 1000;

export function renderSummary(markdown: string, title: string): Promise<RenderedSummary> {
  return invoke<RenderedSummary>('api_render_summary', { markdown, title });
}

/**
 * Copy the summary to the clipboard. Rich text writes HTML plus a plain text fallback,
 * so Outlook and Word keep tables while plain editors still receive readable text.
 */
export async function copySummary(format: CopyFormat, markdown: string, title: string): Promise<void> {
  if (format === 'markdown') {
    await navigator.clipboard.writeText(markdown);
    return;
  }

  const rendered = await renderSummary(markdown, title);
  if (format === 'plain') {
    await navigator.clipboard.writeText(rendered.plainText);
  } else if (format === 'html') {
    await navigator.clipboard.writeText(rendered.html);
  } else {
    await navigator.clipboard.write([
      new ClipboardItem({
        'text/html': new Blob([rendered.html], { type: 'text/html' }),
        'text/plain': new Blob([rendered.plainText], { type: 'text/plain' }),
      }),
    ]);
  }
}

/**
 * Save the summary as a file. PDF opens the system print dialog (choose "Save as PDF" or
 * "Microsoft Print to PDF"); other formats open a save dialog.
 *
 * @returns The saved file path, or null when cancelled or printed
 */
export async function exportSummary(
  format: ExportFormat,
  markdown: string,
  title: string,
  fileName: string
): Promise<string | null> {
  if (format === 'pdf') {
    const rendered = await renderSummary(markdown, title);
    await printHtmlDocument(rendered.htmlDocument);
    return null;
  }
  return invoke<string | null>('api_export_summary', { markdown, title, format, fileName });
}

function printHtmlDocument(html: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const frame = document.createElement('iframe');
    frame.setAttribute('aria-hidden', 'true');
    frame.style.cssText = 'position:fixed;width:0;height:0;border:0;visibility:hidden';
    document.body.appendChild(frame);

    const frameWindow = frame.contentWindow;
    const frameDocument = frame.contentDocument;
    if (!frameWindow || !frameDocument) {
      frame.remove();
      reject(new Error('Unable to prepare document for printing'));
      return;
    }

    frameDocument.open();
    frameDocument.write(html);
    frameDocument.close();

    // The frame must stay in the DOM until the print dialog closes.
    frameWindow.addEventListener('afterprint', () => setTimeout(() => frame.remove(), PRINT_CLEANUP_DELAY_MS), {
      once: true,
    });
    frameWindow.focus();
    frameWindow.print();
    resolve();
  });
}

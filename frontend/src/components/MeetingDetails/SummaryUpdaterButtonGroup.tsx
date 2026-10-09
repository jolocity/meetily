"use client";

import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Copy, Save, Loader2, ChevronDown, FileDown } from 'lucide-react';
import Analytics from '@/lib/analytics';
import { CopyFormat, ExportFormat } from '@/lib/summaryExport';

interface SummaryUpdaterButtonGroupProps {
  isSaving: boolean;
  isDirty: boolean;
  onSave: () => Promise<void>;
  onCopy: (format: CopyFormat) => Promise<void>;
  onExport: (format: ExportFormat) => Promise<void>;
}

const COPY_OPTIONS: { format: CopyFormat; label: string; hint: string }[] = [
  { format: 'rich', label: 'Rich text', hint: 'Outlook, Word, Teams (keeps tables)' },
  { format: 'plain', label: 'Plain text', hint: 'Notepad and plain editors' },
  { format: 'markdown', label: 'Markdown', hint: '.md files, wikis, GitHub' },
  { format: 'html', label: 'HTML source', hint: 'Raw HTML markup' },
];

const EXPORT_OPTIONS: { format: ExportFormat; label: string; hint: string }[] = [
  { format: 'docx', label: 'Word document', hint: '.docx' },
  { format: 'pdf', label: 'PDF', hint: 'Opens the print dialog' },
  { format: 'md', label: 'Markdown', hint: '.md' },
  { format: 'txt', label: 'Plain text', hint: '.txt' },
  { format: 'html', label: 'Web page', hint: '.html' },
  { format: 'rtf', label: 'Rich Text Format', hint: '.rtf' },
];

export function SummaryUpdaterButtonGroup({
  isSaving,
  isDirty,
  onSave,
  onCopy,
  onExport,
}: SummaryUpdaterButtonGroupProps) {
  return (
    <ButtonGroup>
      {/* Save button */}
      <Button
        variant="outline"
        size="sm"
        className={`${isDirty ? 'bg-green-200' : ""}`}
        title={isSaving ? "Saving" : "Save Changes"}
        onClick={() => {
          Analytics.trackButtonClick('save_changes', 'meeting_details');
          onSave();
        }}
        disabled={isSaving}
      >
        {isSaving ? (
          <>
            <Loader2 className="animate-spin" />
            <span className="hidden @[40rem]:inline">Saving...</span>
          </>
        ) : (
          <>
            <Save />
            <span className="hidden @[40rem]:inline">Save</span>
          </>
        )}
      </Button>

      {/* Copy menu */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            title="Copy Summary"
            onClick={() => Analytics.trackButtonClick('copy_summary', 'meeting_details')}
            className="cursor-pointer"
          >
            <Copy />
            <span className="hidden @[40rem]:inline">Copy</span>
            <ChevronDown className="opacity-60" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-64">
          <DropdownMenuLabel>Copy summary as</DropdownMenuLabel>
          <DropdownMenuSeparator />
          {COPY_OPTIONS.map(({ format, label, hint }) => (
            <DropdownMenuItem key={format} onSelect={() => onCopy(format)} className="flex-col items-start gap-0">
              <span>{label}</span>
              <span className="text-xs text-muted-foreground">{hint}</span>
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>

      {/* Save as menu */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            title="Save summary as a file"
            onClick={() => Analytics.trackButtonClick('export_summary', 'meeting_details')}
            className="cursor-pointer"
          >
            <FileDown />
            <span className="hidden @[40rem]:inline">Save as</span>
            <ChevronDown className="opacity-60" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-56">
          <DropdownMenuLabel>Save summary as</DropdownMenuLabel>
          <DropdownMenuSeparator />
          {EXPORT_OPTIONS.map(({ format, label, hint }) => (
            <DropdownMenuItem key={format} onSelect={() => onExport(format)} className="justify-between">
              <span>{label}</span>
              <span className="text-xs text-muted-foreground">{hint}</span>
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </ButtonGroup>
  );
}

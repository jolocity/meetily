import { useCallback, RefObject } from 'react';
import { MeetingSummary, Transcript } from '@/types';
import { BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { toast } from 'sonner';
import Analytics from '@/lib/analytics';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { hasVisibleSummaryContent } from '@/lib/summary-content';
import { CopyFormat, ExportFormat, copySummary, exportSummary } from '@/lib/summaryExport';

const COPY_FORMAT_LABELS: Record<CopyFormat, string> = {
  rich: 'rich text',
  plain: 'plain text',
  markdown: 'Markdown',
  html: 'HTML source',
};

interface UseCopyOperationsProps {
  meeting: any;
  transcripts: Transcript[];
  meetingTitle: string;
  aiSummary: MeetingSummary | null;
  blockNoteSummaryRef: RefObject<BlockNoteSummaryViewRef>;
}

export function useCopyOperations({
  meeting,
  transcripts,
  meetingTitle,
  aiSummary,
  blockNoteSummaryRef,
}: UseCopyOperationsProps) {

  // Helper function to fetch ALL transcripts for copying (not just paginated data)
  const fetchAllTranscripts = useCallback(async (meetingId: string): Promise<Transcript[]> => {
    try {
      console.log('📊 Fetching all transcripts for copying:', meetingId);

      // First, get total count by fetching first page
      const firstPage = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: 1,
        offset: 0,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      const totalCount = firstPage.total_count;
      console.log(`📊 Total transcripts in database: ${totalCount}`);

      if (totalCount === 0) {
        return [];
      }

      // Fetch all transcripts in one call
      const allData = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: totalCount,
        offset: 0,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      console.log(`✅ Fetched ${allData.transcripts.length} transcripts from database for copying`);
      return allData.transcripts;
    } catch (error) {
      console.error('❌ Error fetching all transcripts:', error);
      toast.error('Failed to fetch transcripts for copying');
      return [];
    }
  }, []);

  // Copy transcript to clipboard
  const handleCopyTranscript = useCallback(async () => {
    // CHANGE: Fetch ALL transcripts from database, not from pagination state
    console.log('📊 Fetching all transcripts for copying...');
    const allTranscripts = await fetchAllTranscripts(meeting.id);

    if (!allTranscripts.length) {
      const error_msg = 'No transcripts available to copy';
      console.log(error_msg);
      toast.error(error_msg);
      return;
    }

    console.log(`✅ Copying ${allTranscripts.length} transcripts to clipboard`);

    // Format timestamps as recording-relative [MM:SS] instead of wall-clock time
    const formatTime = (seconds: number | undefined, fallbackTimestamp: string): string => {
      if (seconds === undefined) {
        // For old transcripts without audio_start_time, use wall-clock time
        return fallbackTimestamp;
      }
      const totalSecs = Math.floor(seconds);
      const mins = Math.floor(totalSecs / 60);
      const secs = totalSecs % 60;
      return `[${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`;
    };

    const header = `# Transcript of the Meeting: ${meeting.id} - ${meetingTitle ?? meeting.title}\n\n`;
    const date = `## Date: ${new Date(meeting.created_at).toLocaleDateString()}\n\n`;
    const fullTranscript = allTranscripts
      .map(t => `${formatTime(t.audio_start_time, t.timestamp)} ${t.text}  `)
      .join('\n');

    await navigator.clipboard.writeText(header + date + fullTranscript);
    toast.success("Transcript copied to clipboard");

    // Track copy analytics
    const wordCount = allTranscripts
      .map(t => t.text.split(/\s+/).length)
      .reduce((a, b) => a + b, 0);

    await Analytics.trackCopy('transcript', {
      meeting_id: meeting.id,
      transcript_length: allTranscripts.length.toString(),
      word_count: wordCount.toString()
    });
  }, [meeting, meetingTitle, fetchAllTranscripts]);

  // Build the summary as markdown with a title and date header. Returns null when empty.
  const buildSummaryMarkdown = useCallback(async (): Promise<string | null> => {
    if (!hasVisibleSummaryContent(aiSummary)) {
      return null;
    }
    let summaryMarkdown = '';

    // Try to get markdown from BlockNote editor first
    if (blockNoteSummaryRef.current?.getMarkdown) {
      console.log('📝 Trying to get markdown from ref...');
      summaryMarkdown = await blockNoteSummaryRef.current.getMarkdown();
      console.log('📝 Got markdown from ref, length:', summaryMarkdown.length);
    }

    // Fallback: Check if aiSummary has markdown property
    if (!summaryMarkdown && aiSummary && typeof aiSummary.markdown === 'string') {
      console.log('📝 Using markdown from aiSummary');
      summaryMarkdown = aiSummary.markdown;
      console.log('📝 Markdown from aiSummary, length:', summaryMarkdown.length);
    }

    // Fallback: Check for legacy format
    if (!summaryMarkdown && aiSummary) {
      console.log('📝 Converting legacy format to markdown');
      const sections = Object.entries(aiSummary)
        .filter(([key]) => {
          // Skip non-section keys
          return key !== 'markdown' && key !== 'summary_json' && key !== '_section_order' && key !== 'MeetingName';
        })
        .map(([, section]) => {
          if (section && typeof section === 'object' && 'title' in section && 'blocks' in section) {
            const sectionTitle = `## ${section.title}\n\n`;
            const sectionContent = section.blocks
              .map((block: any) => `- ${block.content}`)
              .join('\n');
            return sectionTitle + sectionContent;
          }
          return '';
        })
        .filter(s => s.trim())
        .join('\n\n');
      summaryMarkdown = sections;
      console.log('📝 Converted legacy format, length:', summaryMarkdown.length);
    }

    if (!summaryMarkdown.trim()) {
      return null;
    }

    const meetingDate = new Date(meeting.created_at).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
    return `# ${meetingTitle}\n\n**Date:** ${meetingDate}\n\n${summaryMarkdown}`;
  }, [aiSummary, meetingTitle, meeting, blockNoteSummaryRef]);

  const handleCopySummary = useCallback(async (format: CopyFormat = 'markdown') => {
    try {
      const markdown = await buildSummaryMarkdown();
      if (!markdown) {
        toast.error('No summary content available to copy');
        return;
      }

      await copySummary(format, markdown, meetingTitle);
      toast.success(`Summary copied as ${COPY_FORMAT_LABELS[format]}`);

      await Analytics.trackCopy('summary', {
        meeting_id: meeting.id,
        format,
        has_markdown: (!!aiSummary && 'markdown' in aiSummary).toString()
      });
    } catch (error) {
      console.error('❌ Failed to copy summary:', error);
      toast.error("Failed to copy summary");
    }
  }, [buildSummaryMarkdown, meetingTitle, meeting, aiSummary]);

  const handleExportSummary = useCallback(async (format: ExportFormat) => {
    try {
      const markdown = await buildSummaryMarkdown();
      if (!markdown) {
        toast.error('No summary content available to save');
        return;
      }

      const meetingDate = new Date(meeting.created_at).toISOString().slice(0, 10);
      const savedPath = await exportSummary(format, markdown, meetingTitle, `${meetingTitle} - ${meetingDate}`);
      if (savedPath) {
        toast.success('Summary saved', { description: savedPath });
      }

      await Analytics.trackButtonClick(`export_summary_${format}`, 'meeting_details');
    } catch (error) {
      console.error('❌ Failed to save summary:', error);
      toast.error('Failed to save summary', { description: String(error) });
    }
  }, [buildSummaryMarkdown, meetingTitle, meeting]);

  return {
    handleCopyTranscript,
    handleCopySummary,
    handleExportSummary,
  };
}

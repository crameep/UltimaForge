/**
 * PatchNotes Component
 *
 * Displays patch notes/changelog when available from the update server.
 * Shows the content from the patch_notes_url specified in the manifest.
 */

import { useState, useEffect, useCallback } from "react";
import "./PatchNotes.css";

interface PatchNotesProps {
  /** URL to fetch patch notes from */
  patchNotesUrl: string | null;
  /** Version being displayed */
  version?: string | null;
  /** Whether to show as collapsed by default */
  defaultCollapsed?: boolean;
}

/**
 * Loading state while fetching patch notes.
 */
function PatchNotesLoading() {
  return (
    <div className="patch-notes-loading">
      <span className="patch-notes-spinner" />
      <span>Loading patch notes...</span>
    </div>
  );
}

/**
 * Error state when patch notes fail to load.
 */
function PatchNotesError({
  error,
  onRetry,
}: {
  error: string;
  onRetry: () => void;
}) {
  return (
    <div className="patch-notes-error">
      <span className="patch-notes-error-icon">!</span>
      <span className="patch-notes-error-text">{error}</span>
      <button className="patch-notes-retry-button" onClick={onRetry}>
        Retry
      </button>
    </div>
  );
}

/**
 * Empty state when no patch notes are available.
 */
function PatchNotesEmpty() {
  return (
    <div className="patch-notes-empty">
      <span className="patch-notes-empty-icon">&#128196;</span>
      <span className="patch-notes-empty-text">No patch notes available.</span>
    </div>
  );
}

/**
 * Parse simple markdown content to HTML.
 * Supports: headers, bold, italic, links, lists, code blocks.
 */
function parseMarkdown(markdown: string): string {
  let html = markdown
    // Escape HTML entities
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    // Headers
    .replace(/^### (.+)$/gm, "<h4>$1</h4>")
    .replace(/^## (.+)$/gm, "<h3>$1</h3>")
    .replace(/^# (.+)$/gm, "<h2>$1</h2>")
    // Bold
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/__(.+?)__/g, "<strong>$1</strong>")
    // Italic
    .replace(/\*(.+?)\*/g, "<em>$1</em>")
    .replace(/_(.+?)_/g, "<em>$1</em>")
    // Inline code
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    // Links
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>')
    // Unordered lists
    .replace(/^\s*[-*]\s+(.+)$/gm, "<li>$1</li>")
    // Line breaks
    .replace(/\n\n/g, "</p><p>")
    .replace(/\n/g, "<br />");

  // Wrap list items in ul tags
  html = html.replace(/(<li>.*?<\/li>)+/g, "<ul>$&</ul>");

  // Wrap in paragraph if not already wrapped
  if (!html.startsWith("<h") && !html.startsWith("<ul")) {
    html = `<p>${html}</p>`;
  }

  return html;
}

/**
 * Main PatchNotes component.
 */
export function PatchNotes({
  patchNotesUrl,
  version,
  defaultCollapsed = false,
}: PatchNotesProps) {
  const [content, setContent] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isCollapsed, setIsCollapsed] = useState(defaultCollapsed);

  /**
   * Fetch patch notes from the URL.
   */
  const fetchPatchNotes = useCallback(async () => {
    if (!patchNotesUrl) {
      setContent(null);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(patchNotesUrl);

      if (!response.ok) {
        throw new Error(`Failed to fetch patch notes: ${response.status}`);
      }

      const text = await response.text();
      setContent(text.trim());
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to load patch notes"
      );
      setContent(null);
    } finally {
      setIsLoading(false);
    }
  }, [patchNotesUrl]);

  // Fetch patch notes when URL changes
  useEffect(() => {
    fetchPatchNotes();
  }, [fetchPatchNotes]);

  // Handle retry
  const handleRetry = () => {
    fetchPatchNotes();
  };

  // Toggle collapsed state
  const toggleCollapsed = () => {
    setIsCollapsed(!isCollapsed);
  };

  // Don't render if no URL provided
  if (!patchNotesUrl) {
    return null;
  }

  return (
    <div className="patch-notes-container">
      <button
        className="patch-notes-header"
        onClick={toggleCollapsed}
        aria-expanded={!isCollapsed}
      >
        <div className="patch-notes-header-content">
          <span className="patch-notes-icon">&#128220;</span>
          <span className="patch-notes-title">
            Patch Notes
            {version && (
              <span className="patch-notes-version">v{version}</span>
            )}
          </span>
        </div>
        <span
          className={`patch-notes-toggle ${isCollapsed ? "collapsed" : ""}`}
        >
          &#9660;
        </span>
      </button>

      {!isCollapsed && (
        <div className="patch-notes-body">
          {isLoading && <PatchNotesLoading />}

          {error && <PatchNotesError error={error} onRetry={handleRetry} />}

          {!isLoading && !error && content && (
            <div
              className="patch-notes-content"
              dangerouslySetInnerHTML={{ __html: parseMarkdown(content) }}
            />
          )}

          {!isLoading && !error && !content && <PatchNotesEmpty />}
        </div>
      )}
    </div>
  );
}

/**
 * Inline patch notes display for compact views.
 */
export function PatchNotesBadge({
  hasNotes,
  onClick,
}: {
  hasNotes: boolean;
  onClick?: () => void;
}) {
  if (!hasNotes) return null;

  return (
    <button className="patch-notes-badge" onClick={onClick} title="View patch notes">
      <span className="patch-notes-badge-icon">&#128220;</span>
      <span className="patch-notes-badge-text">Patch Notes</span>
    </button>
  );
}

import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";

type ExportFormat = "csv" | "md";

interface TranscriptionRow {
  id: string;
  created_at: number;
  original_text: string;
  polished_text: string;
  word_count: number;
  duration_secs: number;
}

const formatTimestamp = (ms: number): string => {
  const date = new Date(ms);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
};

const formatDuration = (seconds: number): string => {
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins}:${secs.toString().padStart(2, "0")}`;
};

const truncate = (text: string, maxLength: number): string => {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength).trimEnd() + "\u2026";
};

/**
 * Rows render on a single truncated line, so a match past the cut would show a
 * result with nothing visibly matching. Slide the window back to the match.
 */
const snippet = (text: string, query: string, maxLength: number): string => {
  if (!query) return truncate(text, maxLength);
  const index = text.toLowerCase().indexOf(query.toLowerCase());
  if (index <= 0) return truncate(text, maxLength);
  const start = Math.max(0, index - 30);
  const prefix = start > 0 ? "\u2026" : "";
  return prefix + truncate(text.slice(start), maxLength);
};

const TranscriptionItem = ({ entry, query }: { entry: TranscriptionRow; query: string }) => {
  return (
    <div className="transcription-row">
      <div className="transcription-row__header">
        <span className="transcription-row__time">
          {formatTimestamp(entry.created_at)}
        </span>
        <span className="transcription-row__meta">
          {entry.word_count} words {"\u00b7"} {formatDuration(entry.duration_secs)}
        </span>
      </div>
      <p className="transcription-row__text">
        {snippet(entry.polished_text, query, 120)}
      </p>
    </div>
  );
};

export const RecentTranscriptions = () => {
  const [transcriptions, setTranscriptions] = useState<TranscriptionRow[]>([]);
  const [query, setQuery] = useState("");
  const [exportError, setExportError] = useState<string | null>(null);
  const [exportedTo, setExportedTo] = useState<string | null>(null);

  const term = query.trim();
  const latestTerm = useRef(term);

  const fetchTranscriptions = useCallback(() => {
    // ponytail: no debounce — the table is local SQLite and the query is
    // index-free but tiny. Add one if histories ever get large enough to lag.
    latestTerm.current = term;
    const request = term
      ? invoke<TranscriptionRow[]>("search_transcriptions", { query: term, limit: 50 })
      : invoke<TranscriptionRow[]>("get_transcriptions", { limit: 20, offset: 0 });

    request
      .then((rows) => {
        // Drop a response that lost the race to a newer keystroke, so the list
        // never shows results for a query the user has already moved past.
        if (latestTerm.current !== term) return;
        setTranscriptions(rows);
      })
      .catch((err: unknown) => {
        console.error("Failed to load transcriptions:", err);
      });
  }, [term]);

  const handleExport = useCallback(async (format: ExportFormat) => {
    setExportError(null);
    setExportedTo(null);
    try {
      const path = await save({
        title: "Export transcription history",
        defaultPath: `talx-history.${format}`,
        filters: [
          {
            name: format === "csv" ? "CSV" : "Markdown",
            extensions: [format],
          },
        ],
      });
      if (path === null) return; // user cancelled
      await invoke("export_history", { format, path });
      setExportedTo(path);
    } catch (err: unknown) {
      setExportError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    fetchTranscriptions();
  }, [fetchTranscriptions]);

  // Re-fetch when the main window saves a new transcription
  useEffect(() => {
    const unlisten = listen("transcription-saved", () => {
      fetchTranscriptions();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchTranscriptions]);

  return (
    <div className="recent-transcriptions">
      <div className="recent-transcriptions__header">
        <h3 className="home-page__section-title">{term ? "Results" : "Recent"}</h3>
        <div className="recent-transcriptions__actions">
          <button
            className="recent-transcriptions__export"
            onClick={() => void handleExport("csv")}
          >
            Export CSV
          </button>
          <button
            className="recent-transcriptions__export"
            onClick={() => void handleExport("md")}
          >
            Export Markdown
          </button>
        </div>
      </div>

      <input
        className="recent-transcriptions__search"
        type="search"
        value={query}
        placeholder="Search transcripts…"
        aria-label="Search transcripts"
        onChange={(event) => setQuery(event.target.value)}
      />

      {exportError && (
        <p className="recent-transcriptions__export-error" role="alert">
          {exportError}
        </p>
      )}
      {exportedTo && (
        <p className="recent-transcriptions__export-ok">Saved to {exportedTo}</p>
      )}

      {transcriptions.length === 0 ? (
        <p className="recent-transcriptions__empty">
          {term
            ? `No transcripts match “${term}”.`
            : "No transcriptions yet. Start recording to see them here."}
        </p>
      ) : (
        <div className="recent-transcriptions__list">
          {transcriptions.map((entry) => (
            <TranscriptionItem key={entry.id} entry={entry} query={term} />
          ))}
        </div>
      )}
    </div>
  );
};

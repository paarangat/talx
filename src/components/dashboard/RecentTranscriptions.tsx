import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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

const TranscriptionItem = ({ entry }: { entry: TranscriptionRow }) => {
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
        {truncate(entry.polished_text, 120)}
      </p>
    </div>
  );
};

export const RecentTranscriptions = () => {
  const [transcriptions, setTranscriptions] = useState<TranscriptionRow[]>([]);

  const fetchTranscriptions = useCallback(() => {
    invoke<TranscriptionRow[]>("get_transcriptions", { limit: 20, offset: 0 })
      .then(setTranscriptions)
      .catch((err: unknown) => {
        console.error("Failed to load transcriptions:", err);
      });
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

  if (transcriptions.length === 0) {
    return (
      <div className="recent-transcriptions">
        <h3 className="home-page__section-title">Recent</h3>
        <p className="recent-transcriptions__empty">
          No transcriptions yet. Start recording to see them here.
        </p>
      </div>
    );
  }

  return (
    <div className="recent-transcriptions">
      <h3 className="home-page__section-title">Recent</h3>
      <div className="recent-transcriptions__list">
        {transcriptions.map((entry) => (
          <TranscriptionItem key={entry.id} entry={entry} />
        ))}
      </div>
    </div>
  );
};

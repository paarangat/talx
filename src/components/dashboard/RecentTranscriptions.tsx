import { useTranscriptions } from "../../hooks/useSessionStore";
import type { TranscriptionEntry } from "../../stores/sessionStore";

const formatTimestamp = (ts: number): string => {
  const date = new Date(ts);
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

const TranscriptionRow = ({ entry }: { entry: TranscriptionEntry }) => {
  return (
    <div className="transcription-row">
      <div className="transcription-row__header">
        <span className="transcription-row__time">
          {formatTimestamp(entry.timestamp)}
        </span>
        <span className="transcription-row__meta">
          {entry.wordCount} words {"\u00b7"} {formatDuration(entry.durationSeconds)}
        </span>
      </div>
      <p className="transcription-row__text">
        {truncate(entry.polishedText, 120)}
      </p>
    </div>
  );
};

export const RecentTranscriptions = () => {
  const transcriptions = useTranscriptions();

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
          <TranscriptionRow key={entry.id} entry={entry} />
        ))}
      </div>
    </div>
  );
};

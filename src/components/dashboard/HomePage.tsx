import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSessionStats } from "../../hooks/useSessionStore";
import { useRecordingControl } from "../../hooks/useRecordingControl";
import { sessionStore } from "../../stores/sessionStore";
import {
  computeStreak,
  formatTimeSaved,
  timeSavedSeconds,
  wordsPerMinute,
  TYPING_WPM,
} from "../../lib/metrics";
import { StatCard } from "./StatCard";
import { ProviderStatus } from "./ProviderStatus";
import { RecentTranscriptions } from "./RecentTranscriptions";

const formatDuration = (totalSeconds: number): string => {
  const mins = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;
  return `${mins}:${secs.toString().padStart(2, "0")}`;
};

interface HomePageProps {
  onNavigate: (page: "settings") => void;
}

export const HomePage = ({ onNavigate }: HomePageProps) => {
  const stats = useSessionStats();
  const { error, isProcessing, isRecording, toggleRecording } = useRecordingControl();
  const [streak, setStreak] = useState(0);

  // Every tile is derived from these two reads, so they refresh together — the
  // widget records in a separate window, and its in-memory store is not shared
  // with this one.
  useEffect(() => {
    const refresh = () => {
      const now = new Date();
      const startOfDay = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
      invoke<{ words: number; recording_secs: number; sessions: number }>("get_today_stats", { sinceMs: startOfDay })
        .then((dbStats) => {
          sessionStore.loadFromDb(dbStats);
        })
        .catch((err: unknown) => {
          console.error("Failed to load today stats:", err);
        });

      // Bounded: a streak longer than 2000 days is not worth paging a whole
      // history over IPC to count.
      invoke<number[]>("get_transcription_timestamps", { limit: 2000 })
        .then((timestamps) => {
          setStreak(computeStreak(timestamps));
        })
        .catch((err: unknown) => {
          console.error("Failed to load transcription timestamps:", err);
        });
    };

    refresh();
    const unlisten = listen("transcription-saved", refresh);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const pace = wordsPerMinute(stats.wordsToday, stats.recordingSecondsToday);
  const saved = Math.round(timeSavedSeconds(stats.wordsToday, stats.recordingSecondsToday));

  return (
    <div className="home-page">
      <h2 className="home-page__title">Home</h2>

      <div className="home-page__quick-action">
        <button
          className="home-page__record-btn"
          onClick={toggleRecording}
          disabled={isProcessing}
        >
          <span className="home-page__record-dot" />
          {isProcessing ? "Processing..." : isRecording ? "Stop Recording" : "Start Recording"}
        </button>
        {error && (
          <p className="home-page__record-error" role="alert">
            {error}
          </p>
        )}
      </div>

      <section className="home-page__stats">
        <StatCard
          label="Words today"
          value={stats.wordsToday.toLocaleString()}
        />
        <StatCard
          label="Recording time"
          value={formatDuration(stats.recordingSecondsToday)}
        />
        <StatCard
          label="Sessions"
          value={stats.sessionsToday.toString()}
        />
        <StatCard
          label="Words / min"
          value={pace === null ? "—" : pace.toString()}
        />
        <StatCard
          label="Time saved"
          value={formatTimeSaved(saved)}
        />
        <StatCard
          label="Streak"
          value={streak.toString()}
          unit={streak === 1 ? "day" : "days"}
        />
      </section>

      <p className="home-page__stats-note">
        Pace and time saved cover today only, estimated against an average typing
        speed of {TYPING_WPM} WPM. Streak counts consecutive days with at least one
        transcription.
      </p>

      <ProviderStatus onConfigure={() => onNavigate("settings")} />

      <RecentTranscriptions />
    </div>
  );
};

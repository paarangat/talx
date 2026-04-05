import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSessionStats } from "../../hooks/useSessionStore";
import { useRecordingControl } from "../../hooks/useRecordingControl";
import { sessionStore } from "../../stores/sessionStore";
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

  useEffect(() => {
    const now = new Date();
    const startOfDay = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    invoke<{ words: number; recording_secs: number; sessions: number }>("get_today_stats", { sinceMs: startOfDay })
      .then((dbStats) => {
        sessionStore.loadFromDb(dbStats);
      })
      .catch((err: unknown) => {
        console.error("Failed to load today stats:", err);
      });
  }, []);

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
      </section>

      <ProviderStatus onConfigure={() => onNavigate("settings")} />

      <RecentTranscriptions />
    </div>
  );
};

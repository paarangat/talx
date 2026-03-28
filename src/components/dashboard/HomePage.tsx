import { emit } from "@tauri-apps/api/event";
import { useSessionStats } from "../../hooks/useSessionStore";
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

  const handleStartRecording = () => {
    emit("start-recording").catch((err: unknown) => {
      console.error("Failed to emit start-recording:", err);
    });
  };

  return (
    <div className="home-page">
      <h2 className="home-page__title">Home</h2>

      <div className="home-page__quick-action">
        <button className="home-page__record-btn" onClick={handleStartRecording}>
          <span className="home-page__record-dot" />
          Start Recording
        </button>
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

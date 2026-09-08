import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSessionStats } from "../../hooks/useSessionStore";
import { useRecordingControl } from "../../hooks/useRecordingControl";
import { sessionStore } from "../../stores/sessionStore";
import { DEFAULT_HOTKEY, HOTKEY_KEY, formatHotkey } from "../../lib/settings";
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

const today = () =>
  new Date().toLocaleDateString(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
  });

interface HomePageProps {
  onNavigate: (page: "settings") => void;
}

export const HomePage = ({ onNavigate }: HomePageProps) => {
  const stats = useSessionStats();
  const { error, isProcessing, isRecording, toggleRecording } = useRecordingControl();
  const [streak, setStreak] = useState(0);
  const [hotkey, setHotkey] = useState(DEFAULT_HOTKEY);

  useEffect(() => {
    setHotkey(localStorage.getItem(HOTKEY_KEY) ?? DEFAULT_HOTKEY);
  }, []);

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
  const badge = formatHotkey(hotkey);

  return (
    <div className="home">
      <header className="masthead">
        <p className="masthead__date">{today()}</p>
        {/* The headline states what happened, so it has to change when
            nothing has. */}
        <h2 className="masthead__title">
          {stats.wordsToday > 0 ? (
            <>
              Today you spoke <em>instead of typing</em>
            </>
          ) : (
            <>
              Nothing dictated yet — <em>press and speak</em>
            </>
          )}
        </h2>
      </header>

      {/* One figure carries the page. The five supporting readings sit on a
          rail beneath it, subordinate by size rather than by position. */}
      <section className="ledger">
        <div className="ledger__lead">
          <span className="ledger__figure">{stats.wordsToday.toLocaleString()}</span>
          <span className="ledger__caption">
            {stats.wordsToday === 1 ? "word" : "words"} dictated
          </span>
        </div>

        <div className="ledger__rail">
          <StatCard label="Recording" value={formatDuration(stats.recordingSecondsToday)} />
          <StatCard label="Sessions" value={stats.sessionsToday.toString()} />
          <StatCard label="Words / min" value={pace === null ? "—" : pace.toString()} />
          <StatCard label="Time saved" value={formatTimeSaved(saved)} />
          <StatCard
            label="Streak"
            value={streak.toString()}
            unit={streak === 1 ? "day" : "days"}
          />
        </div>
      </section>

      <div className="home__action">
        <button
          className={`record ${isRecording ? "record--live" : ""}`}
          onClick={toggleRecording}
          disabled={isProcessing}
        >
          <span className="record__dot" />
          {isProcessing ? "Processing…" : isRecording ? "Stop dictating" : "Start dictating"}
          <kbd className="record__kbd">{badge}</kbd>
        </button>
        {error && (
          <p className="home__error" role="alert">
            {error}
          </p>
        )}
        <p className="home__note">
          Pace and time saved cover today only, measured against an average
          typing speed of {TYPING_WPM} WPM. Streak counts consecutive days with
          at least one transcription.
        </p>
      </div>

      <ProviderStatus onConfigure={() => onNavigate("settings")} />

      <RecentTranscriptions />
    </div>
  );
};

import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { IdlePill } from "./components/IdlePill";
import { RecordingCard } from "./components/RecordingCard";
import { ResultCard } from "./components/ResultCard";
import { useFirstUse } from "./hooks/useFirstUse";
import { sessionStore } from "./stores/sessionStore";

type WidgetState = "idle" | "recording" | "result";

const WINDOW_SIZES: Record<WidgetState, { width: number; height: number }> = {
  idle: { width: 200, height: 60 },
  recording: { width: 300, height: 300 },
  result: { width: 300, height: 280 },
};

const MOCK_TRANSCRIPT =
  "Main bhi soch raha tha ki we should probably refactor the authentication module before the sprint ends";

export const App = () => {
  const [state, setState] = useState<WidgetState>("idle");
  const [launching, setLaunching] = useState(true);
  const [timer, setTimer] = useState("0:00");
  const [transcript, setTranscript] = useState("");
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const transcriptRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const secondsRef = useRef(0);
  const isRecordingRef = useRef(false);

  const [isFirstRecording, markRecordingUsed] = useFirstUse("recording");
  const [isFirstStop, markStopUsed] = useFirstUse("stop");

  // Entrance animation
  useEffect(() => {
    const timeout = setTimeout(() => setLaunching(false), 700);
    return () => clearTimeout(timeout);
  }, []);

  // Resize window on state change
  useEffect(() => {
    const size = WINDOW_SIZES[state];
    invoke("resize_window", { width: size.width, height: size.height }).catch(
      (err: unknown) => {
        console.error("Failed to resize window:", err);
      },
    );
  }, [state]);

  // Sync tray status
  useEffect(() => {
    invoke("update_tray_status", { status: state }).catch((err: unknown) => {
      console.error("Failed to update tray status:", err);
    });
  }, [state]);

  const clearTimers = useCallback(() => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
    if (transcriptRef.current) {
      clearTimeout(transcriptRef.current);
      transcriptRef.current = null;
    }
  }, []);

  const formatTime = (totalSeconds: number) => {
    const mins = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  };

  const simulateTranscript = useCallback(() => {
    const words = MOCK_TRANSCRIPT.split(" ");
    let wordIndex = 0;

    const addWord = () => {
      if (!isRecordingRef.current) return;
      if (wordIndex < words.length) {
        const word = words[wordIndex];
        if (word !== undefined) {
          setTranscript((prev) => (prev ? `${prev} ${word}` : word));
        }
        wordIndex++;
        const delay = 120 + Math.random() * 180;
        transcriptRef.current = setTimeout(addWord, delay);
      }
    };

    transcriptRef.current = setTimeout(addWord, 300);
  }, []);

  const handleStartRecording = useCallback(() => {
    setState("recording");
    isRecordingRef.current = true;
    setTranscript("");
    secondsRef.current = 0;
    setTimer("0:00");
    markRecordingUsed();

    timerRef.current = setInterval(() => {
      secondsRef.current += 1;
      setTimer(formatTime(secondsRef.current));
    }, 1000);

    simulateTranscript();
  }, [simulateTranscript, markRecordingUsed]);

  const handleStopRecording = useCallback(() => {
    isRecordingRef.current = false;
    clearTimers();
    markStopUsed();

    const durationSeconds = secondsRef.current;
    const currentTranscript = transcript;
    const wordCount = currentTranscript.split(/\s+/).filter(Boolean).length;

    sessionStore.addTranscription({
      timestamp: Date.now(),
      originalText: currentTranscript,
      polishedText: currentTranscript,
      wordCount,
      durationSeconds,
    });

    setState("result");
  }, [clearTimers, markStopUsed, transcript]);

  const handleDismiss = useCallback(() => {
    setState("idle");
  }, []);

  const handlePaste = useCallback(() => {
    navigator.clipboard.writeText(transcript).then(() => {
      invoke("paste_to_focused_app").catch((err: unknown) => {
        console.error("Failed to paste to focused app:", err);
      });
    }).catch((err: unknown) => {
      console.error("Failed to write to clipboard:", err);
    });
    setState("idle");
  }, [transcript]);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(transcript).catch((err: unknown) => {
      console.error("Failed to copy to clipboard:", err);
    });
    setState("idle");
  }, [transcript]);

  useEffect(() => {
    return clearTimers;
  }, [clearTimers]);

  return (
    <div className={`widget-wrapper ${launching ? "widget-wrapper--launching" : ""}`}>
      {state === "idle" && (
        <IdlePill onActivate={handleStartRecording} showHint={isFirstRecording} />
      )}
      {state === "recording" && (
        <RecordingCard
          transcript={transcript}
          timer={timer}
          onStop={handleStopRecording}
          showStopHint={isFirstStop}
        />
      )}
      {state === "result" && (
        <ResultCard
          text={transcript}
          diffRanges={[]}
          wordCount={transcript.split(/\s+/).filter(Boolean).length}
          latencyMs={0}
          cost="$0.00"
          onPaste={handlePaste}
          onCopy={handleCopy}
          onDismiss={handleDismiss}
        />
      )}
    </div>
  );
};

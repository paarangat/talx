import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { IdlePill } from "./components/IdlePill";
import { RecordingCard } from "./components/RecordingCard";
import { ResultCard } from "./components/ResultCard";
import { useFirstUse } from "./hooks/useFirstUse";

type WidgetState = "idle" | "recording" | "result";

const WINDOW_SIZES: Record<WidgetState, { width: number; height: number }> = {
  idle: { width: 200, height: 60 },
  recording: { width: 300, height: 300 },
  result: { width: 300, height: 280 },
};

const MOCK_TRANSCRIPT =
  "Main bhi soch raha tha ki we should probably refactor the authentication module before the sprint ends";

const MOCK_RESULT = {
  text: "I was thinking we should probably refactor the authentication module before the sprint ends.",
  diffRanges: [{ start: 27, end: 44 }],
  wordCount: 42,
  latencyMs: 620,
  cost: "$0.0003",
};

export const App = () => {
  const [state, setState] = useState<WidgetState>("idle");
  const [launching, setLaunching] = useState(true);
  const [timer, setTimer] = useState("0:00");
  const [transcript, setTranscript] = useState("");
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const transcriptRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const secondsRef = useRef(0);

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
      () => {},
    );
  }, [state]);

  // Sync tray status
  useEffect(() => {
    invoke("update_tray_status", { status: state }).catch(() => {});
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
    clearTimers();
    markStopUsed();
    setState("result");
  }, [clearTimers, markStopUsed]);

  const handleDismiss = useCallback(() => {
    setState("idle");
  }, []);

  const handlePaste = useCallback(() => {
    setState("idle");
  }, []);

  const handleCopy = useCallback(() => {
    setState("idle");
  }, []);

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
          text={MOCK_RESULT.text}
          diffRanges={MOCK_RESULT.diffRanges}
          wordCount={MOCK_RESULT.wordCount}
          latencyMs={MOCK_RESULT.latencyMs}
          cost={MOCK_RESULT.cost}
          onPaste={handlePaste}
          onCopy={handleCopy}
          onDismiss={handleDismiss}
        />
      )}
    </div>
  );
};

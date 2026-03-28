import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { IdlePill } from "./components/IdlePill";
import { RecordingCard } from "./components/RecordingCard";
import { ResultCard } from "./components/ResultCard";
import { useFirstUse } from "./hooks/useFirstUse";
import { sessionStore } from "./stores/sessionStore";
import { computeDiffRanges } from "./utils/diffRanges";

type WidgetState = "idle" | "recording" | "result";

const WINDOW_SIZES: Record<WidgetState, { width: number; height: number }> = {
  idle: { width: 200, height: 60 },
  recording: { width: 300, height: 300 },
  result: { width: 300, height: 280 },
};

export const App = () => {
  const [state, setState] = useState<WidgetState>("idle");
  const [launching, setLaunching] = useState(true);
  const [timer, setTimer] = useState("0:00");
  const [transcript, setTranscript] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [polishing, setPolishing] = useState(false);
  const [polishFailed, setPolishFailed] = useState(false);
  const [polishModel, setPolishModel] = useState("");
  const [polishLatencyMs, setPolishLatencyMs] = useState(0);
  const [diffRanges, setDiffRanges] = useState<Array<{ start: number; end: number }>>([]);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
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

  const formatTime = (totalSeconds: number) => {
    const mins = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  };

  const clearTimer = useCallback(() => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  const handleStartRecording = useCallback(() => {
    setTranscript("");
    setError(null);
    markRecordingUsed();

    invoke("start_recording").catch((err: unknown) => {
      console.error("Failed to start recording:", err);
      setError(String(err));
      setTimeout(() => setError(null), 3000);
    });
  }, [markRecordingUsed]);

  const handleStopRecording = useCallback(() => {
    clearTimer();
    markStopUsed();

    invoke("stop_recording").catch((err: unknown) => {
      console.error("Failed to stop recording:", err);
      setError(String(err));
      setState("idle");
      setTimeout(() => setError(null), 3000);
    });
  }, [clearTimer, markStopUsed]);

  const handleDismiss = useCallback(() => {
    invoke("dismiss_result").catch(() => {});
    setState("idle");
  }, []);

  const handlePaste = useCallback(() => {
    invoke("dismiss_result").catch(() => {});
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
    invoke("dismiss_result").catch(() => {});
    navigator.clipboard.writeText(transcript).catch((err: unknown) => {
      console.error("Failed to copy to clipboard:", err);
    });
    setState("idle");
  }, [transcript]);

  // Listen for Tauri events from Rust backend
  useEffect(() => {
    const unlisteners = [
      listen("recording-started", () => {
        setState("recording");
        secondsRef.current = 0;
        setTimer("0:00");

        timerRef.current = setInterval(() => {
          secondsRef.current += 1;
          setTimer(formatTime(secondsRef.current));
        }, 1000);
      }),

      listen<string>("transcription-partial", (event) => {
        setTranscript(event.payload);
      }),

      listen<string>("transcription-result", (event) => {
        clearTimer();
        const rawText = event.payload;
        setTranscript(rawText);
        setPolishing(true);
        setPolishFailed(false);

        const durationSeconds = secondsRef.current;
        const wordCount = rawText.split(/\s+/).filter(Boolean).length;

        invoke<{ polished_text: string; model: string; latency_ms: number }>(
          "polish_transcript",
          { text: rawText },
        )
          .then((result) => {
            setTranscript(result.polished_text);
            setPolishModel(result.model);
            setPolishLatencyMs(result.latency_ms);
            setDiffRanges(computeDiffRanges(rawText, result.polished_text));
            setPolishFailed(false);

            sessionStore.addTranscription({
              timestamp: Date.now(),
              originalText: rawText,
              polishedText: result.polished_text,
              wordCount: result.polished_text.split(/\s+/).filter(Boolean).length,
              durationSeconds,
            });

            setPolishing(false);
            setState("result");
          })
          .catch((err: unknown) => {
            console.error("Polishing failed:", err);
            setPolishFailed(true);
            setPolishModel("raw");
            setPolishLatencyMs(0);
            setDiffRanges([]);

            sessionStore.addTranscription({
              timestamp: Date.now(),
              originalText: rawText,
              polishedText: rawText,
              wordCount,
              durationSeconds,
            });

            setPolishing(false);
            setState("result");
          });
      }),

      listen<string>("recording-error", (event) => {
        clearTimer();
        setError(event.payload);
        setState("idle");
        setTimeout(() => setError(null), 4000);
      }),
    ];

    return () => {
      for (const unlisten of unlisteners) {
        unlisten.then((fn) => fn());
      }
    };
  }, [clearTimer]);

  // Cleanup timer on unmount
  useEffect(() => {
    return clearTimer;
  }, [clearTimer]);

  return (
    <div className={`widget-wrapper ${launching ? "widget-wrapper--launching" : ""}`}>
      {error && (
        <div className="widget-error">
          <span className="widget-error__text">{error}</span>
        </div>
      )}
      {!error && state === "idle" && (
        <IdlePill onActivate={handleStartRecording} showHint={isFirstRecording} />
      )}
      {!error && state === "recording" && (
        <RecordingCard
          transcript={transcript}
          timer={timer}
          onStop={handleStopRecording}
          showStopHint={isFirstStop}
          polishing={polishing}
        />
      )}
      {!error && state === "result" && (
        <ResultCard
          text={transcript}
          diffRanges={diffRanges}
          wordCount={transcript.split(/\s+/).filter(Boolean).length}
          latencyMs={polishLatencyMs}
          cost="$0.00"
          model={polishModel}
          polished={!polishFailed}
          onPaste={handlePaste}
          onCopy={handleCopy}
          onDismiss={handleDismiss}
        />
      )}
    </div>
  );
};

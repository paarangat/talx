import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";
import { Widget } from "./components/Widget";
import type { WidgetState } from "./components/Widget";
import { AUTO_PASTE_KEY, DEFAULT_HOTKEY, HOTKEY_KEY } from "./lib/settings";
import { subscribeToUiErrors } from "./lib/uiErrors";
import { sessionStore } from "./stores/sessionStore";

const WINDOW_SIZES: Record<WidgetState, { width: number; height: number }> = {
  idle: { width: 120, height: 36 },
  recording: { width: 220, height: 36 },
  polishing: { width: 120, height: 36 },
  success: { width: 120, height: 36 },
};

export const App = () => {
  const [state, setState] = useState<WidgetState>("idle");
  const [launching, setLaunching] = useState(true);
  const [timer, setTimer] = useState("0:00");
  const [error, setError] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const secondsRef = useRef(0);
  const polishGenRef = useRef(0);
  const successTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const errorTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Entrance animation
  useEffect(() => {
    const timeout = setTimeout(() => setLaunching(false), 700);
    return () => clearTimeout(timeout);
  }, []);

  // Resize window on state change
  useEffect(() => {
    const size = WINDOW_SIZES[state];
    const prevState = document.body.dataset.widgetState;

    // When shrinking (recording → polishing/idle), delay resize for CSS transition
    const isShrinking =
      prevState === "recording" && state !== "recording";

    const doResize = () => {
      invoke("resize_window", { width: size.width, height: size.height }).catch(
        (err: unknown) => {
          console.error("Failed to resize window:", err);
        },
      );
    };

    if (isShrinking) {
      setTimeout(doResize, 350);
    } else {
      doResize();
    }

    document.body.dataset.widgetState = state;
  }, [state]);

  // Sync tray status
  useEffect(() => {
    invoke("update_tray_status", { status: state }).catch((err: unknown) => {
      console.error("Failed to update tray status:", err);
    });
  }, [state]);

  // Sync persisted hotkey to Rust backend on mount
  useEffect(() => {
    const savedHotkey = localStorage.getItem(HOTKEY_KEY);
    if (savedHotkey && savedHotkey !== DEFAULT_HOTKEY) {
      invoke("set_hotkey", { hotkey: savedHotkey }).catch((err: unknown) => {
        console.error("Failed to sync hotkey:", err);
        localStorage.setItem(HOTKEY_KEY, DEFAULT_HOTKEY);
        invoke("set_hotkey", { hotkey: DEFAULT_HOTKEY }).catch(
          (fallbackErr: unknown) => {
            console.error("Failed to restore default hotkey:", fallbackErr);
          },
        );
      });
    }
  }, []);

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

  const clearSuccessTimeout = useCallback(() => {
    if (successTimeoutRef.current) {
      clearTimeout(successTimeoutRef.current);
      successTimeoutRef.current = null;
    }
  }, []);

  const clearErrorTimeout = useCallback(() => {
    if (errorTimeoutRef.current) {
      clearTimeout(errorTimeoutRef.current);
      errorTimeoutRef.current = null;
    }
  }, []);

  const showError = useCallback(
    (message: string) => {
      clearErrorTimeout();
      setError(message);
      errorTimeoutRef.current = setTimeout(() => {
        setError(null);
        errorTimeoutRef.current = null;
      }, 4000);
    },
    [clearErrorTimeout],
  );

  useEffect(() => {
    return subscribeToUiErrors((messages) => {
      if (messages.length > 0) {
        showError(messages.join(" | "));
      }
    });
  }, [showError]);

  // Listen for Tauri events from Rust backend
  useEffect(() => {
    const unlisteners = [
      listen("recording-started", () => {
        // Cancel any pending success timeout
        clearSuccessTimeout();

        setState("recording");
        secondsRef.current = 0;
        setTimer("0:00");
        setError(null);
        polishGenRef.current += 1;

        timerRef.current = setInterval(() => {
          secondsRef.current += 1;
          setTimer(formatTime(secondsRef.current));
        }, 1000);
      }),

      listen<string>("transcription-result", (event) => {
        clearTimer();
        const rawText = event.payload;
        const durationSeconds = secondsRef.current;

        // Empty transcription — go straight to idle
        if (!rawText.trim()) {
          invoke("dismiss_result").catch((err: unknown) => {
            console.error("Failed to dismiss empty transcription result:", err);
            showError("Failed to reset the widget after an empty transcription.");
          });
          setState("idle");
          return;
        }

        setState("polishing");
        const gen = polishGenRef.current;

        const handleResult = (text: string) => {
          if (polishGenRef.current !== gen) return;

          const wordCount = text.split(/\s+/).filter(Boolean).length;

          if (localStorage.getItem(AUTO_PASTE_KEY) !== "false") {
            invoke("paste_to_focused_app", { text }).catch((err: unknown) => {
              console.error("Failed to paste to focused app:", err);
            });
          }

          // Save transcription (fire-and-forget)
          invoke("save_transcription", {
            originalText: rawText,
            polishedText: text,
            wordCount,
            durationSecs: durationSeconds,
          })
            .then(() => {
              emit("transcription-saved").catch((err: unknown) => {
                console.error("Failed to notify dashboard about saved transcription:", err);
                showError("Transcription saved, but the dashboard did not refresh automatically.");
              });
            })
            .catch((err: unknown) => {
              console.error("Failed to save transcription:", err);
            });

          sessionStore.addStats({ wordCount, durationSeconds });

          setState("success");
          successTimeoutRef.current = setTimeout(() => {
            invoke("dismiss_result").catch((err: unknown) => {
              console.error("Failed to dismiss result:", err);
            });
            setState("idle");
          }, 1500);
        };

        invoke<{ polished_text: string; model: string; latency_ms: number }>(
          "polish_transcript",
          { text: rawText },
        )
          .then((result) => {
            if (polishGenRef.current !== gen) return;
            handleResult(result.polished_text);
          })
          .catch((err: unknown) => {
            if (polishGenRef.current !== gen) return;
            console.error("Polishing failed, using raw text:", err);
            handleResult(rawText);
          });
      }),

      listen<string>("recording-error", (event) => {
        clearTimer();
        showError(event.payload);
        invoke("dismiss_result").catch((err: unknown) => {
          console.error("Failed to dismiss errored transcription result:", err);
          showError("Failed to reset the widget after the recording error.");
        });
        setState("idle");
      }),
    ];

    return () => {
      for (const unlisten of unlisteners) {
        unlisten.then((fn) => fn());
      }
    };
  }, [clearTimer, clearSuccessTimeout, showError]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      clearTimer();
      clearSuccessTimeout();
      clearErrorTimeout();
    };
  }, [clearErrorTimeout, clearTimer, clearSuccessTimeout]);

  return (
    <div className={`widget-wrapper ${launching ? "widget-wrapper--launching" : ""}`}>
      <Widget state={state} timer={timer} error={error} />
    </div>
  );
};

import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";
import { Widget } from "./components/Widget";
import type { PolishResult, WidgetState } from "./components/Widget";
import { AUTO_PASTE_KEY, DEFAULT_HOTKEY, HOTKEY_KEY } from "./lib/settings";
import {
  getErrorMessage,
  getPasteErrorMessage,
  PASTE_ACCESSIBILITY_DENIED,
  subscribeToUiErrors,
} from "./lib/uiErrors";
import { sessionStore } from "./stores/sessionStore";

// Window = widget + the wrapper padding that gives the contact shadow room
// to fall (8px top / 12px sides / 16px bottom, see .widget-wrapper).
const PAD_X = 24;
const PAD_Y = 24;
const sized = (width: number, height: number) => ({
  width: width + PAD_X,
  height: height + PAD_Y,
});

const WINDOW_SIZES: Record<WidgetState, { width: number; height: number }> = {
  idle: sized(176, 38),
  // Recording and polishing share one card so the handoff is a cross-fade
  // rather than a resize — the moment the transcript lands is the wrong
  // time to make the window jump.
  recording: sized(300, 122),
  polishing: sized(300, 122),
  success: sized(300, 148),
};

/** The card grows a transcript pane the first time a partial streams in.
 *  Only Soniox streams, so on the default provider it never does. */
const STREAMING_CARD = sized(300, 204);

const SUCCESS_WITH_COPY = sized(300, 192);

// Errors replace the widget contents with a message, so the window has to be
// wide enough to read it regardless of which state we were in.
const ERROR_SIZE = sized(300, 38);

/** How long the result card stays up. Long enough to actually read the text —
 *  the old 1.5s was tuned for a checkmark, not a card. */
const RESULT_MS = 2600;
/** Longer when a Copy button is the only route to the text. */
const RESULT_WITH_COPY_MS = 7000;

export const App = () => {
  const [state, setState] = useState<WidgetState>("idle");
  const [launching, setLaunching] = useState(true);
  const [timer, setTimer] = useState("0:00");
  const [error, setError] = useState<string | null>(null);
  const [partial, setPartial] = useState("");
  const [result, setResult] = useState<PolishResult | null>(null);
  const [copied, setCopied] = useState(false);
  const [autoPaste, setAutoPaste] = useState(true);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const secondsRef = useRef(0);
  const polishGenRef = useRef(0);
  const successTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const errorTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastSizeRef = useRef(WINDOW_SIZES.idle);

  // Entrance animation
  useEffect(() => {
    const timeout = setTimeout(() => setLaunching(false), 700);
    return () => clearTimeout(timeout);
  }, []);

  // Resize window on state change.
  //
  // Direction matters: growing has to happen BEFORE the CSS transition or the
  // card is clipped by the old window bounds mid-animation, and shrinking has
  // to happen AFTER it or the card is clipped on the way down. The old code
  // special-cased only "leaving recording", which broke as soon as more than
  // one state had a size of its own.
  useEffect(() => {
    const showCopy = state === "success" && !autoPaste && result !== null;
    const streaming =
      partial.length > 0 && (state === "recording" || state === "polishing");
    const size = error
      ? ERROR_SIZE
      : showCopy
        ? SUCCESS_WITH_COPY
        : streaming
          ? STREAMING_CARD
          : WINDOW_SIZES[state];

    const doResize = () => {
      invoke("resize_window", { width: size.width, height: size.height }).catch(
        (err: unknown) => {
          console.error("Failed to resize window:", err);
        },
      );
    };

    const previous = lastSizeRef.current;
    // `partial` is a dependency, so a streaming transcript re-runs this on
    // every token. Without this guard that is 5–20 resize_window round trips a
    // second, each repositioning an always-on-top window — which also fights
    // the user dragging the widget while they dictate.
    if (size.width === previous.width && size.height === previous.height) return;

    lastSizeRef.current = size;
    const isShrinking = size.height < previous.height || size.width < previous.width;

    if (isShrinking) {
      const id = setTimeout(doResize, 320);
      return () => clearTimeout(id);
    }
    doResize();
  }, [state, error, autoPaste, result, partial]);

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
      listen<string | null>("recording-started", (event) => {
        // Cancel any pending success timeout
        clearSuccessTimeout();

        setState("recording");
        secondsRef.current = 0;
        setTimer("0:00");
        setError(null);
        setPartial("");
        setResult(null);
        setCopied(false);
        setAutoPaste(localStorage.getItem(AUTO_PASTE_KEY) !== "false");
        polishGenRef.current += 1;

        // Payload is set when the chosen mic was missing and we fell back to the default.
        if (event.payload) {
          showError(event.payload);
        }

        timerRef.current = setInterval(() => {
          secondsRef.current += 1;
          setTimer(formatTime(secondsRef.current));
        }, 1000);
      }),

      // Emitted by the Soniox streaming session. Groq Whisper is batch, so
      // this never fires on the default provider and the card falls back to
      // its listening state — which is why that state has to look finished.
      listen<string>("transcription-partial", (event) => {
        setPartial(event.payload);
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

        const handleResult = (text: string, model: string, latencyMs: number) => {
          if (polishGenRef.current !== gen) return;

          const wordCount = text.split(/\s+/).filter(Boolean).length;
          const pasteEnabled = localStorage.getItem(AUTO_PASTE_KEY) !== "false";
          setAutoPaste(pasteEnabled);
          setResult({ text, model, latencyMs, wordCount });

          if (pasteEnabled) {
            invoke("paste_to_focused_app", { text }).catch((err: unknown) => {
              console.error("Failed to paste to focused app:", err);
              showError(getPasteErrorMessage(err));

              // The permission can only be granted in System Settings, and the
              // pill is too small to explain how — send the user to the panel
              // in the dashboard that has the button.
              if (getErrorMessage(err, "") === PASTE_ACCESSIBILITY_DENIED) {
                invoke("open_dashboard_window", { section: "settings" }).catch(
                  (openErr: unknown) => {
                    console.error("Failed to open settings window:", openErr);
                  },
                );
              }
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
          successTimeoutRef.current = setTimeout(
            () => {
              invoke("dismiss_result").catch((err: unknown) => {
                console.error("Failed to dismiss result:", err);
              });
              setState("idle");
            },
            pasteEnabled ? RESULT_MS : RESULT_WITH_COPY_MS,
          );
        };

        invoke<{ polished_text: string; model: string; latency_ms: number }>(
          "polish_transcript",
          { text: rawText },
        )
          .then((polished) => {
            if (polishGenRef.current !== gen) return;
            handleResult(polished.polished_text, polished.model, polished.latency_ms);
          })
          .catch((err: unknown) => {
            if (polishGenRef.current !== gen) return;
            console.error("Polishing failed, using raw text:", err);
            // The raw transcript is still the user's words, so it still ships —
            // labelled as unpolished rather than silently attributed to a model.
            handleResult(rawText, "unpolished", 0);
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

  const showCopy = state === "success" && !autoPaste && result !== null;

  const handleCopy = useCallback(() => {
    if (!result) return;
    invoke("copy_to_clipboard", { text: result.text })
      .then(() => setCopied(true))
      .catch((err: unknown) => {
        console.error("Failed to copy result:", err);
        showError(getErrorMessage(err, "Could not copy to the clipboard."));
      });
  }, [result, showError]);

  return (
    <div className={`widget-wrapper ${launching ? "widget-wrapper--launching" : ""}`}>
      <Widget
        state={state}
        timer={timer}
        error={error}
        partial={partial}
        result={result}
        onCopy={showCopy ? handleCopy : null}
        copied={copied}
      />
    </div>
  );
};

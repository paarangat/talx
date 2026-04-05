import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { getErrorMessage } from "../lib/uiErrors";

export type RecordingPhase = "idle" | "recording" | "processing";

const START_TIMEOUT_MS = 5_000;
const STOP_TIMEOUT_MS = 30_000;

export const useRecordingControl = () => {
  const [phase, setPhase] = useState<RecordingPhase>("idle");
  const [error, setError] = useState<string | null>(null);
  const processingTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingActionRef = useRef<"start" | "stop" | null>(null);

  const clearProcessingTimeout = useCallback(() => {
    if (processingTimeoutRef.current) {
      clearTimeout(processingTimeoutRef.current);
      processingTimeoutRef.current = null;
    }

    pendingActionRef.current = null;
  }, []);

  const scheduleProcessingTimeout = useCallback((action: "start" | "stop") => {
    clearProcessingTimeout();
    pendingActionRef.current = action;

    processingTimeoutRef.current = setTimeout(() => {
      const timedOutAction = pendingActionRef.current;
      clearProcessingTimeout();
      setPhase("idle");
      setError(
        timedOutAction === "stop"
          ? "Transcription is taking longer than expected. It may still finish in the background."
          : "Recording did not start. Please check your microphone and try again.",
      );
    }, action === "stop" ? STOP_TIMEOUT_MS : START_TIMEOUT_MS);
  }, [clearProcessingTimeout]);

  useEffect(() => {
    const unlisteners = [
      listen("recording-started", () => {
        clearProcessingTimeout();
        setPhase("recording");
        setError(null);
      }),
      listen("transcription-result", () => {
        clearProcessingTimeout();
        setPhase("idle");
        setError(null);
      }),
      listen<string>("recording-error", (event) => {
        clearProcessingTimeout();
        setPhase("idle");
        setError(event.payload || "Recording failed. Please try again.");
      }),
    ];

    return () => {
      clearProcessingTimeout();
      for (const unlisten of unlisteners) {
        unlisten.then((fn) => fn());
      }
    };
  }, [clearProcessingTimeout]);

  const toggleRecording = useCallback(async () => {
    if (phase === "processing") {
      return;
    }

    const stopping = phase === "recording";
    setError(null);
    setPhase("processing");
    scheduleProcessingTimeout(stopping ? "stop" : "start");

    try {
      if (stopping) {
        await invoke("stop_recording");
      } else {
        await invoke("start_recording");
      }
    } catch (err) {
      clearProcessingTimeout();
      const message = `Failed to ${stopping ? "stop" : "start"} recording: ${getErrorMessage(
        err,
        "Unknown error.",
      )}`;
      console.error(message, err);
      setError(message);
      setPhase(stopping ? "recording" : "idle");
    }
  }, [clearProcessingTimeout, phase, scheduleProcessingTimeout]);

  return {
    phase,
    error,
    isProcessing: phase === "processing",
    isRecording: phase === "recording",
    toggleRecording,
  };
};

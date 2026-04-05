import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export type RecordingPhase = "idle" | "recording" | "processing";

export const useRecordingControl = () => {
  const [phase, setPhase] = useState<RecordingPhase>("idle");

  useEffect(() => {
    const unlisteners = [
      listen("recording-started", () => {
        setPhase("recording");
      }),
      listen("transcription-result", () => {
        setPhase("idle");
      }),
      listen("recording-error", () => {
        setPhase("idle");
      }),
    ];

    return () => {
      for (const unlisten of unlisteners) {
        unlisten.then((fn) => fn());
      }
    };
  }, []);

  const toggleRecording = useCallback(async () => {
    if (phase === "processing") {
      return;
    }

    const stopping = phase === "recording";
    setPhase("processing");

    try {
      if (stopping) {
        await invoke("stop_recording");
      } else {
        await invoke("start_recording");
      }
    } catch (err) {
      console.error(`Failed to ${stopping ? "stop" : "start"} recording:`, err);
      setPhase(stopping ? "recording" : "idle");
    }
  }, [phase]);

  return {
    phase,
    isProcessing: phase === "processing",
    isRecording: phase === "recording",
    toggleRecording,
  };
};

import { useSyncExternalStore } from "react";
import { sessionStore } from "../stores/sessionStore";
import type { SessionStats, TranscriptionEntry } from "../stores/sessionStore";

export const useSessionStats = (): SessionStats => {
  return useSyncExternalStore(
    sessionStore.subscribe,
    sessionStore.getStats,
  );
};

export const useTranscriptions = (): TranscriptionEntry[] => {
  return useSyncExternalStore(
    sessionStore.subscribe,
    sessionStore.getTranscriptions,
  );
};

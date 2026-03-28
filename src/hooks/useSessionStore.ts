import { useSyncExternalStore } from "react";
import { sessionStore } from "../stores/sessionStore";
import type { SessionStats } from "../stores/sessionStore";

export const useSessionStats = (): SessionStats => {
  return useSyncExternalStore(
    sessionStore.subscribe,
    sessionStore.getStats,
  );
};

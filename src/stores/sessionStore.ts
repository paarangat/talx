type Listener = () => void;

interface SessionStats {
  wordsToday: number;
  recordingSecondsToday: number;
  sessionsToday: number;
}

const state: SessionStats = {
  wordsToday: 0,
  recordingSecondsToday: 0,
  sessionsToday: 0,
};

const listeners = new Set<Listener>();

const notify = () => {
  for (const listener of listeners) {
    listener();
  }
};

export const sessionStore = {
  getStats(): SessionStats {
    return state;
  },

  addStats(entry: { wordCount: number; durationSeconds: number }): void {
    state.wordsToday += entry.wordCount;
    state.recordingSecondsToday += entry.durationSeconds;
    state.sessionsToday += 1;
    notify();
  },

  subscribe(listener: Listener): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
};

export type { SessionStats };

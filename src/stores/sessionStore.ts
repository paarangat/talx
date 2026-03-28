type Listener = () => void;

interface TranscriptionEntry {
  id: string;
  timestamp: number;
  originalText: string;
  polishedText: string;
  wordCount: number;
  durationSeconds: number;
}

interface SessionStats {
  wordsToday: number;
  recordingSecondsToday: number;
  sessionsToday: number;
}

interface SessionState {
  stats: SessionStats;
  transcriptions: TranscriptionEntry[];
}

const state: SessionState = {
  stats: {
    wordsToday: 0,
    recordingSecondsToday: 0,
    sessionsToday: 0,
  },
  transcriptions: [],
};

const listeners = new Set<Listener>();

const notify = () => {
  for (const listener of listeners) {
    listener();
  }
};

export const sessionStore = {
  getStats(): SessionStats {
    return state.stats;
  },

  getTranscriptions(): TranscriptionEntry[] {
    return state.transcriptions;
  },

  addTranscription(entry: Omit<TranscriptionEntry, "id">): void {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    state.transcriptions.unshift({ ...entry, id });
    if (state.transcriptions.length > 20) {
      state.transcriptions.pop();
    }
    state.stats.wordsToday += entry.wordCount;
    state.stats.recordingSecondsToday += entry.durationSeconds;
    state.stats.sessionsToday += 1;
    notify();
  },

  subscribe(listener: Listener): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
};

export type { TranscriptionEntry, SessionStats };

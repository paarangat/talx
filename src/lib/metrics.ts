/** Average sustained typing speed used as the baseline for "time saved". */
export const TYPING_WPM = 40;

/**
 * Speaking pace. Returns null when nothing has been recorded yet so the caller
 * can render a dash instead of NaN/Infinity.
 */
export const wordsPerMinute = (words: number, recordedSeconds: number): number | null => {
  if (recordedSeconds <= 0 || words <= 0) return null;
  return Math.round((words / recordedSeconds) * 60);
};

/**
 * Seconds saved versus typing the same words at {@link TYPING_WPM}.
 * Clamped at zero — dictating slower than 40 WPM saves nothing, it does not
 * cost negative time.
 */
export const timeSavedSeconds = (words: number, recordedSeconds: number): number => {
  if (words <= 0) return 0;
  return Math.max(0, (words / TYPING_WPM) * 60 - recordedSeconds);
};

/**
 * Human-readable "time saved". Deliberately not the mm:ss used for recording
 * time — saved time regularly runs past an hour, where "95:00" reads as a
 * stopwatch rather than as an hour and a half.
 */
export const formatTimeSaved = (seconds: number): string => {
  const totalMinutes = Math.round(seconds / 60);
  if (totalMinutes < 60) return `${totalMinutes}m`;
  return `${Math.floor(totalMinutes / 60)}h ${totalMinutes % 60}m`;
};

/**
 * Local-calendar-day key for an epoch-millisecond timestamp.
 *
 * Deliberately built from the local getters rather than `Math.floor(ms / 86400000)`:
 * the latter buckets by UTC, which would roll the streak over at the wrong hour
 * for every user outside UTC.
 */
const dayKey = (ms: number): string => {
  const d = new Date(ms);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
};

/**
 * Consecutive local days with at least one transcription.
 *
 * A streak counts as live if the most recent entry was today *or* yesterday,
 * so it does not read as 0 every morning before the first dictation of the day.
 */
export const computeStreak = (timestampsMs: number[], now: number = Date.now()): number => {
  if (timestampsMs.length === 0) return 0;

  const days = new Set(timestampsMs.map(dayKey));

  // Walk backwards from noon so that adding/subtracting a day never lands on a
  // DST discontinuity around midnight.
  const cursor = new Date(now);
  cursor.setHours(12, 0, 0, 0);

  if (!days.has(dayKey(cursor.getTime()))) {
    cursor.setDate(cursor.getDate() - 1);
    if (!days.has(dayKey(cursor.getTime()))) return 0;
  }

  let streak = 0;
  while (days.has(dayKey(cursor.getTime()))) {
    streak += 1;
    cursor.setDate(cursor.getDate() - 1);
  }
  return streak;
};

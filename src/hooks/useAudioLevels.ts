import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Bars in the meter. Also the length of the rolling history. */
export const METER_BARS = 34;

/** ~18fps. Fast enough to track syllables, slow enough that the poll is free. */
const POLL_MS = 55;

const SILENCE: number[] = Array.from({ length: METER_BARS }, () => 0);

/**
 * Rolling history of real microphone loudness, newest sample last.
 *
 * The old waveform was `Math.random()` baked once into a CSS keyframe, so the
 * bars danced identically whether you were talking or the mic was muted. This
 * polls the actual input level and scrolls it right-to-left like a seismograph,
 * which is the whole point of putting a meter on a dictation tool.
 */
export const useAudioLevels = (active: boolean): number[] => {
  const [levels, setLevels] = useState<number[]>(SILENCE);
  // Kept in a ref so the interval never re-subscribes on every new sample.
  const historyRef = useRef<number[]>(SILENCE);

  useEffect(() => {
    if (!active) {
      historyRef.current = SILENCE;
      setLevels(SILENCE);
      return;
    }

    let cancelled = false;

    const tick = async () => {
      let level = 0;
      try {
        level = await invoke<number>("get_audio_level");
      } catch {
        // A dropped poll is not worth surfacing — the meter simply reads as
        // silence for one frame and the next tick recovers.
        level = 0;
      }
      if (cancelled) return;
      historyRef.current = [...historyRef.current.slice(1), level];
      setLevels(historyRef.current);
    };

    const id = setInterval(tick, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [active]);

  return levels;
};

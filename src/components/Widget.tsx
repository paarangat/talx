import { useMemo } from "react";
import { useDrag } from "../hooks/useDrag";

type WidgetState = "idle" | "recording" | "polishing" | "success";

interface WidgetProps {
  state: WidgetState;
  timer: string;
  error: string | null;
}

const BAR_COUNT = 16;

export const Widget = ({ state, timer, error }: WidgetProps) => {
  const onDragStart = useDrag();

  const bars = useMemo(
    () =>
      Array.from({ length: BAR_COUNT }, (_, i) => ({
        key: i,
        height: 4 + Math.random() * 10,
        delay: Math.random() * 0.5,
        duration: 0.35 + Math.random() * 0.4,
      })),
    [],
  );

  return (
    <div
      className={`widget widget--${state} ${error ? "widget--error" : ""}`}
      onMouseDown={onDragStart}
    >
      {/* Grain texture overlay */}
      <div className="widget__grain" />

      {/* Idle: logo pill */}
      {state === "idle" && (
        <div className="widget__logo">
          <div className="widget__logo-dot" />
          <span className="widget__logo-text">Talx</span>
        </div>
      )}

      {/* Recording: rec dot + waveform + timer */}
      {state === "recording" && (
        <div className="widget__recording">
          <div className="widget__rec-dot" />
          <div className="widget__waveform">
            {bars.map((bar) => (
              <div
                key={bar.key}
                className="widget__bar"
                style={{
                  ["--h" as string]: `${bar.height}px`,
                  animationDelay: `${bar.delay}s`,
                  animationDuration: `${bar.duration}s`,
                }}
              />
            ))}
          </div>
          <span className="widget__timer">{timer}</span>
        </div>
      )}

      {/* Polishing: pulsing dot */}
      {state === "polishing" && <div className="widget__pulse-dot" />}

      {/* Success: sage checkmark */}
      {state === "success" && (
        <div className="widget__check">
          <svg viewBox="0 0 12 12" fill="none">
            <path
              d="M2.5 6L5 8.5L9.5 3.5"
              stroke="#fff"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>
      )}

      {/* Error: terracotta ring overlay */}
      {error && (
        <div className="widget__error-ring" title={error} />
      )}
    </div>
  );
};

export type { WidgetState };

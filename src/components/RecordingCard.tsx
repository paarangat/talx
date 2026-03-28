import { useMemo } from "react";
import { DragHandle } from "./DragHandle";
import { useDrag } from "../hooks/useDrag";

interface RecordingCardProps {
  transcript: string;
  timer: string;
  onStop: () => void;
  showStopHint: boolean;
}

const BAR_COUNT = 40;

export const RecordingCard = ({ transcript, timer, onStop, showStopHint }: RecordingCardProps) => {
  const onDragStart = useDrag();

  const bars = useMemo(
    () =>
      Array.from({ length: BAR_COUNT }, (_, i) => ({
        key: i,
        height: 6 + Math.random() * 22,
        delay: Math.random() * 0.6,
        duration: 0.4 + Math.random() * 0.5,
      })),
    [],
  );

  return (
    <div className="recording-card" onClick={onStop} onMouseDown={onDragStart}>
      <DragHandle />

      {/* Header */}
      <div className="recording-card__header">
        <div className="recording-card__status">
          <div className="recording-card__dot-container">
            <div className="recording-card__dot" />
            <div className="recording-card__dot-ping" />
          </div>
          <span className="recording-card__label">Recording</span>
        </div>
        <span className="recording-card__timer">{timer}</span>
      </div>

      {/* Waveform */}
      <div className="waveform">
        {bars.map((bar) => (
          <div
            key={bar.key}
            className="waveform__bar"
            style={{
              ["--h" as string]: `${bar.height}px`,
              animationDelay: `${bar.delay}s`,
              animationDuration: `${bar.duration}s`,
            }}
          />
        ))}
      </div>

      {/* Transcript */}
      <div className="recording-card__transcript">
        <p className="recording-card__transcript-text">
          {transcript}
          <span className="cursor-blink" />
        </p>
      </div>

      {/* Footer */}
      <div className="recording-card__footer">
        <span className="lang-pill">
          <span className="lang-pill__dot" />
          Hindi + English
        </span>
        <span className={`stop-hint ${!showStopHint ? "shortcut-hint--hidden" : ""}`}>
          ⌥Space to stop
        </span>
      </div>
    </div>
  );
};

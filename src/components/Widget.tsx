import { useEffect, useState } from "react";
import { useDrag } from "../hooks/useDrag";
import { METER_BARS, useAudioLevels } from "../hooks/useAudioLevels";
import { DEFAULT_HOTKEY, HOTKEY_KEY, formatHotkey } from "../lib/settings";

type WidgetState = "idle" | "recording" | "polishing" | "success";

interface PolishResult {
  text: string;
  model: string;
  latencyMs: number;
  wordCount: number;
}

interface WidgetProps {
  state: WidgetState;
  timer: string;
  error: string | null;
  /** Live partial transcript. Only Soniox streams; Groq is batch, so this
   *  stays empty for the default provider and the card shows a mic state. */
  partial: string;
  result: PolishResult | null;
  /** Shown only when auto-paste is off, where the text would otherwise
   *  live nowhere the user can reach it. */
  onCopy: (() => void) | null;
  copied: boolean;
}

/** Bar heights in px. Silence still shows a hairline so the meter reads as
 *  present-and-listening rather than broken. */
const BAR_MIN = 3;
const BAR_MAX = 26;

const Meter = ({ levels }: { levels: number[] }) => (
  <div className="meter" aria-hidden="true">
    {levels.map((level, i) => (
      <span
        key={i}
        className="meter__bar"
        style={{
          height: `${BAR_MIN + level * (BAR_MAX - BAR_MIN)}px`,
          // Newest sample (right edge) leads; the ramp is gentle because a
          // steep one made the trailing history read as a signal dying out
          // rather than as history.
          opacity: 0.6 + (i / METER_BARS) * 0.4,
        }}
      />
    ))}
  </div>
);

export const Widget = ({
  state,
  timer,
  error,
  partial,
  result,
  onCopy,
  copied,
}: WidgetProps) => {
  const onDragStart = useDrag();
  const levels = useAudioLevels(state === "recording");
  const [hotkey, setHotkey] = useState(DEFAULT_HOTKEY);

  useEffect(() => {
    setHotkey(localStorage.getItem(HOTKEY_KEY) ?? DEFAULT_HOTKEY);
  }, [state]);

  const isCard = !error && (state === "recording" || state === "polishing" || state === "success");

  // Groq Whisper is batch, so most sessions never stream a partial. Rather
  // than hold open a transcript pane that stays empty for the default
  // provider, the card starts compact and grows the first time text arrives.
  const hasStream = !error && partial.length > 0;

  return (
    <div
      // An error replaces the state rather than decorating it. Emitting both
      // let `.widget--recording`'s 122px height win the cascade over the pill
      // while the window had been resized to 38px — the mic-fallback notice
      // arrives mid-recording, so that path clipped the message by 60px.
      className={`widget widget--${error ? "error" : state} ${
        isCard ? "widget--card" : "widget--pill"
      } ${hasStream ? "widget--streaming" : ""} ${onCopy ? "widget--has-copy" : ""}`}
      onMouseDown={onDragStart}
    >
      <div className="widget__grain" />

      {/* Errors take the whole surface — the widget is too small to show a
          state and a message, and the message is the actionable part. */}
      {error && (
        <div className="widget__body">
          <span className="widget__err-mark" />
          <span className="widget__err-text" title={error}>
            {error}
          </span>
        </div>
      )}

      {!error && state === "idle" && (
        <div className="widget__body">
          <span className="brand">
            <span className="brand__dot" />
            <span className="brand__name">Talx</span>
          </span>
          <kbd className="kbd">{formatHotkey(hotkey)}</kbd>
        </div>
      )}

      {!error && (state === "recording" || state === "polishing") && (
        <div className="card">
          <div className="card__head">
            {state === "recording" ? (
              <span className="status status--rec">
                <span className="status__dot">
                  <span className="status__ping" />
                </span>
                Recording
              </span>
            ) : (
              <span className="status status--polish">
                <span className="status__dot status__dot--still" />
                Polishing
              </span>
            )}
            <span className="card__timer">{timer}</span>
          </div>

          {state === "recording" ? (
            <Meter levels={levels} />
          ) : (
            <div className="shimmer" aria-hidden="true">
              <span className="shimmer__sweep" />
            </div>
          )}

          {/* Bottom-aligned with a fade at the top: streaming text scrolls up
              under the mask, so the newest words are always the visible ones
              and the card never needs a scrollbar. */}
          {hasStream && (
            <div className="card__transcript">
              <p className="card__text">
                {partial}
                {state === "recording" && <span className="caret" />}
              </p>
            </div>
          )}

          <div className="card__foot">
            <span className="chip">
              <span className="chip__dot" />
              Hindi + English
            </span>
            <span className="card__hint">
              <kbd className="kbd kbd--bare">{formatHotkey(hotkey)}</kbd> to stop
            </span>
          </div>
        </div>
      )}

      {!error && state === "success" && (
        <div className="card">
          <div className="card__head">
            <span className="status status--done">
              <span className="check">
                <svg viewBox="0 0 12 12" fill="none">
                  <path
                    d="M2.5 6.2L4.8 8.5L9.5 3.8"
                    stroke="currentColor"
                    strokeWidth="1.9"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </span>
              <span className="status__label">Polished</span>
            </span>
            {result && <span className="badge">{result.model}</span>}
          </div>

          <div className="card__result">
            <p className="card__text">{result?.text}</p>
          </div>

          {onCopy && (
            <button
              type="button"
              className={`btn ${copied ? "btn--done" : ""}`}
              onClick={onCopy}
              onMouseDown={(e) => e.stopPropagation()}
            >
              {copied ? "Copied" : "Copy"}
            </button>
          )}

          {result && (
            <div className="card__foot card__foot--stats">
              <span>{result.wordCount} words</span>
              <span className="card__sep">·</span>
              <span>{result.latencyMs}ms</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export type { WidgetState, PolishResult };

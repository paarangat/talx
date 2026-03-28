import { DragHandle } from "./DragHandle";
import { useDrag } from "../hooks/useDrag";

interface ResultCardProps {
  text: string;
  diffRanges: Array<{ start: number; end: number }>;
  wordCount: number;
  latencyMs: number;
  cost: string;
  model: string;
  polished: boolean;
  onPaste: () => void;
  onCopy: () => void;
  onDismiss: () => void;
}

export const ResultCard = ({
  text,
  diffRanges,
  wordCount,
  latencyMs,
  cost,
  model,
  polished,
  onPaste,
  onCopy,
  onDismiss,
}: ResultCardProps) => {
  const onDragStart = useDrag();

  const renderText = () => {
    if (diffRanges.length === 0) {
      return <>{text}</>;
    }

    const parts: Array<{ content: string; highlighted: boolean }> = [];
    let lastIndex = 0;

    for (const range of diffRanges) {
      if (range.start > lastIndex) {
        parts.push({ content: text.slice(lastIndex, range.start), highlighted: false });
      }
      parts.push({ content: text.slice(range.start, range.end), highlighted: true });
      lastIndex = range.end;
    }

    if (lastIndex < text.length) {
      parts.push({ content: text.slice(lastIndex), highlighted: false });
    }

    return (
      <>
        {parts.map((part, i) =>
          part.highlighted ? (
            <span key={i} className="result-card__diff">
              {part.content}
            </span>
          ) : (
            <span key={i}>{part.content}</span>
          ),
        )}
      </>
    );
  };

  return (
    <div className="result-card" onMouseDown={onDragStart}>
      <DragHandle />

      {/* Header */}
      <div className="result-card__header">
        <div className="result-card__status">
          <div className={`result-card__check ${polished ? "" : "result-card__check--unpolished"}`}>
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
          <span className={`result-card__label ${polished ? "" : "result-card__label--unpolished"}`}>
            {polished ? "Polished" : "Unpolished"}
          </span>
        </div>
        <span className="result-card__model">{model}</span>
      </div>

      {/* Content */}
      <div className="result-card__content">
        <p className="result-card__text">{renderText()}</p>
      </div>

      {/* Actions */}
      <div className="result-card__actions">
        <button
          className="result-card__btn result-card__btn--paste"
          onClick={onPaste}
        >
          Paste
        </button>
        <button
          className="result-card__btn result-card__btn--copy"
          onClick={onCopy}
        >
          Copy
        </button>
        <button
          className="result-card__btn result-card__btn--dismiss"
          onClick={onDismiss}
        >
          Dismiss
        </button>
      </div>

      {/* Stats */}
      <div className="result-card__footer">
        <span className="result-card__stat">{wordCount} words</span>
        <span className="result-card__stat">·</span>
        <span className="result-card__stat">{latencyMs}ms</span>
        <span className="result-card__stat">·</span>
        <span className="result-card__stat">{cost}</span>
      </div>
    </div>
  );
};

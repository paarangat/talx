import { invoke } from "@tauri-apps/api/core";
import { useDrag } from "../hooks/useDrag";

interface IdlePillProps {
  onActivate: () => void;
  showHint: boolean;
}

export const IdlePill = ({ onActivate, showHint }: IdlePillProps) => {
  const onDragStart = useDrag();

  const handleSettings = (e: React.MouseEvent) => {
    e.stopPropagation();
    invoke("open_dashboard_window", { section: "settings" }).catch((err: unknown) => {
      console.error("Failed to open dashboard window:", err);
    });
  };

  return (
    <div className="idle-pill" onClick={onActivate} onMouseDown={onDragStart}>
      <div className="idle-pill__dot" />
      <span className="idle-pill__text">Talx</span>
      <button className="idle-pill__settings" onClick={handleSettings} aria-label="Settings">
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="8" cy="8" r="2.5" />
          <path d="M13.5 8a5.5 5.5 0 0 0-.1-.8l1.3-1-.7-1.2-1.5.5a5.5 5.5 0 0 0-1.2-.7L11 3.3h-1.4l-.3 1.5a5.5 5.5 0 0 0-1.2.7l-1.5-.5-.7 1.2 1.3 1a5.5 5.5 0 0 0 0 1.6l-1.3 1 .7 1.2 1.5-.5c.4.3.7.5 1.2.7l.3 1.5H11l.3-1.5c.4-.2.8-.4 1.2-.7l1.5.5.7-1.2-1.3-1a5.5 5.5 0 0 0 .1-.8Z" />
        </svg>
      </button>
      {showHint && (
        <span className="idle-pill__hotkey">
          ⌥ Space
        </span>
      )}
    </div>
  );
};

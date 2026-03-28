import { useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const useDrag = () => {
  const onMouseDown = useCallback((e: React.MouseEvent) => {
    // Don't drag if clicking on a button or interactive element
    const target = e.target as HTMLElement;
    if (target.closest("button")) return;

    e.preventDefault();
    getCurrentWindow().startDragging().catch((err: unknown) => {
      console.error("Failed to start dragging:", err);
    });
  }, []);

  return onMouseDown;
};

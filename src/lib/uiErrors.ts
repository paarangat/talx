type UiErrorListener = (messages: string[]) => void;

const listeners = new Set<UiErrorListener>();
let messages: string[] = [];

export const getErrorMessage = (
  error: unknown,
  fallback = "Something went wrong.",
): string => {
  if (typeof error === "string" && error.trim()) {
    return error;
  }

  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string" &&
    error.message.trim()
  ) {
    return error.message;
  }

  return fallback;
};

/**
 * Machine-readable failure codes returned verbatim by `paste_to_focused_app`.
 * All user-facing copy for them lives here, not in Rust.
 */
export const PASTE_ACCESSIBILITY_DENIED = "ACCESSIBILITY_DENIED";
export const PASTE_CLIPBOARD_ONLY = "CLIPBOARD_ONLY";

export const getPasteErrorMessage = (error: unknown): string => {
  switch (getErrorMessage(error, "")) {
    case PASTE_ACCESSIBILITY_DENIED:
      // Kept short so it fits the pill; the settings window that opens
      // alongside it carries the full explanation.
      return "Permission needed — press ⌘V";
    case PASTE_CLIPBOARD_ONLY:
      return "Copied — press ⌘V to paste";
    default:
      return "Paste failed.";
  }
};

export const reportUiError = (message: string) => {
  const nextMessage = message.trim();
  if (!nextMessage || messages.includes(nextMessage)) {
    return;
  }

  messages = [...messages, nextMessage];

  for (const listener of listeners) {
    listener(messages);
  }
};

export const subscribeToUiErrors = (listener: UiErrorListener) => {
  listeners.add(listener);
  listener(messages);

  return () => {
    listeners.delete(listener);
  };
};

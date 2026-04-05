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

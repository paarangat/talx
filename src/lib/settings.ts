export const ASR_PROVIDER_KEY = "talx:asr-provider";
export const LLM_PROVIDER_KEY = "talx:llm-provider";
export const AUTO_PASTE_KEY = "talx:auto-paste";
export const HOTKEY_KEY = "talx:hotkey";
export const ASR_MODEL_KEY = "talx:asr-model";
export const LLM_MODEL_KEY = "talx:llm-model";

export const DEFAULT_ASR_PROVIDER = "groq";
export const DEFAULT_LLM_PROVIDER = "groq";
export const DEFAULT_ASR_MODEL = "whisper-large-v3-turbo";
export const DEFAULT_LLM_MODEL = "llama-3.3-70b-versatile";

const platformInfo =
  typeof navigator === "undefined"
    ? ""
    : `${navigator.userAgent} ${navigator.platform}`.toLowerCase();

export const DEFAULT_HOTKEY = platformInfo.includes("windows")
  ? "ctrl+shift+space"
  : "alt+space";

const KEY_GLYPHS: Record<string, string> = {
  alt: "⌥",
  option: "⌥",
  ctrl: "⌃",
  control: "⌃",
  shift: "⇧",
  cmd: "⌘",
  command: "⌘",
  meta: "⌘",
  space: "Space",
};

/**
 * "alt+space" → "⌥Space". The widget has room for a glyph badge but not for
 * the word "alt", and glyphs are what the user sees on their own keyboard.
 */
export const formatHotkey = (hotkey: string): string =>
  hotkey
    .split("+")
    .map((part) => {
      const key = part.trim().toLowerCase();
      return KEY_GLYPHS[key] ?? key.charAt(0).toUpperCase() + key.slice(1);
    })
    .join("");

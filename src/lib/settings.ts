export const ASR_PROVIDER_KEY = "talx:asr-provider";
export const LLM_PROVIDER_KEY = "talx:llm-provider";
export const AUTO_PASTE_KEY = "talx:auto-paste";
export const LANGUAGE_KEY = "talx:language";
export const HOTKEY_KEY = "talx:hotkey";
export const ASR_MODEL_KEY = "talx:asr-model";
export const LLM_MODEL_KEY = "talx:llm-model";

export const DEFAULT_ASR_PROVIDER = "groq";
export const DEFAULT_LLM_PROVIDER = "groq";
export const DEFAULT_ASR_MODEL = "whisper-large-v3-turbo";
export const DEFAULT_LLM_MODEL = "llama-3.3-70b-versatile";
export const DEFAULT_LANGUAGE = "hi+en";

const platformInfo =
  typeof navigator === "undefined"
    ? ""
    : `${navigator.userAgent} ${navigator.platform}`.toLowerCase();

export const DEFAULT_HOTKEY = platformInfo.includes("windows")
  ? "ctrl+shift+space"
  : "alt+space";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

const ASR_PROVIDER_KEY = "talx:asr-provider";
const LLM_PROVIDER_KEY = "talx:llm-provider";
const AUTO_PASTE_KEY = "talx:auto-paste";
const LANGUAGE_KEY = "talx:language";

export const GeneralTab = () => {
  const [autoPaste, setAutoPaste] = useState(() => {
    const stored = localStorage.getItem(AUTO_PASTE_KEY);
    return stored !== null ? stored === "true" : true;
  });
  const [language, setLanguage] = useState(() => {
    return localStorage.getItem(LANGUAGE_KEY) ?? "hi+en";
  });
  const [asrProvider, setAsrProvider] = useState(() => {
    return localStorage.getItem(ASR_PROVIDER_KEY) ?? "groq";
  });
  const [llmProvider, setLlmProvider] = useState(() => {
    return localStorage.getItem(LLM_PROVIDER_KEY) ?? "groq";
  });

  // Sync provider to Rust on mount and change
  useEffect(() => {
    invoke("set_asr_provider", { provider: asrProvider }).catch((err: unknown) => {
      console.error("Failed to set ASR provider:", err);
    });
  }, [asrProvider]);

  useEffect(() => {
    invoke("set_llm_provider", { provider: llmProvider }).catch((err: unknown) => {
      console.error("Failed to set LLM provider:", err);
    });
  }, [llmProvider]);

  const handleProviderChange = (value: string) => {
    setAsrProvider(value);
    localStorage.setItem(ASR_PROVIDER_KEY, value);
  };

  const handleLlmProviderChange = (value: string) => {
    setLlmProvider(value);
    localStorage.setItem(LLM_PROVIDER_KEY, value);
  };

  return (
    <div className="settings-tab">
      <h2 className="settings-tab__title">General</h2>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Hotkey</h3>
        <div className="settings-tab__row">
          <div className="settings-tab__row-info">
            <span className="settings-tab__label">Activation shortcut</span>
            <span className="settings-tab__description">Press to start/stop recording</span>
          </div>
          <div className="settings-tab__row-action">
            <span className="settings-tab__hotkey-badge">⌥ Space</span>
            <button className="settings-tab__btn-secondary">Change</button>
          </div>
        </div>
      </section>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Speech Provider</h3>
        <div className="settings-tab__row">
          <div className="settings-tab__row-info">
            <span className="settings-tab__label">Transcription engine</span>
            <span className="settings-tab__description">
              {asrProvider === "groq"
                ? "Groq Whisper — free, batch transcription after recording stops"
                : "Soniox — paid, real-time streaming transcription"}
            </span>
          </div>
          <select
            className="settings-select"
            value={asrProvider}
            onChange={(e) => handleProviderChange(e.target.value)}
          >
            <option value="groq">Groq Whisper (Free)</option>
            <option value="soniox">Soniox (Paid)</option>
          </select>
        </div>
      </section>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Polish Provider</h3>
        <div className="settings-tab__row">
          <div className="settings-tab__row-info">
            <span className="settings-tab__label">LLM engine</span>
            <span className="settings-tab__description">
              {llmProvider === "groq"
                ? "Groq LLM — free transcript polishing via Llama 3.3"
                : "OpenAI — paid, high-quality transcript polishing"}
            </span>
          </div>
          <select
            className="settings-select"
            value={llmProvider}
            onChange={(e) => handleLlmProviderChange(e.target.value)}
          >
            <option value="groq">Groq LLM (Free)</option>
            <option value="openai">OpenAI (Paid)</option>
          </select>
        </div>
      </section>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Behavior</h3>
        <div className="settings-tab__row">
          <div className="settings-tab__row-info">
            <span className="settings-tab__label">Auto-paste after polish</span>
            <span className="settings-tab__description">
              Automatically paste polished text into the active app
            </span>
          </div>
          <button
            className={`settings-toggle ${autoPaste ? "settings-toggle--on" : ""}`}
            onClick={() => {
              const next = !autoPaste;
              setAutoPaste(next);
              localStorage.setItem(AUTO_PASTE_KEY, String(next));
            }}
            role="switch"
            aria-checked={autoPaste}
          >
            <div className="settings-toggle__knob" />
          </button>
        </div>
      </section>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Language</h3>
        <div className="settings-tab__row">
          <div className="settings-tab__row-info">
            <span className="settings-tab__label">Recognition language</span>
            <span className="settings-tab__description">Language(s) for speech recognition</span>
          </div>
          <select
            className="settings-select"
            value={language}
            onChange={(e) => {
              setLanguage(e.target.value);
              localStorage.setItem(LANGUAGE_KEY, e.target.value);
            }}
          >
            <option value="en">English</option>
            <option value="hi+en">Hindi + English</option>
          </select>
        </div>
      </section>
    </div>
  );
};

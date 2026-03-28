import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ApiKeyFieldProps {
  label: string;
  description: string;
  value: string;
  onChange: (value: string) => void;
  provider: string;
}

const ApiKeyField = ({ label, description, value, onChange, provider }: ApiKeyFieldProps) => {
  const [visible, setVisible] = useState(false);
  const [testStatus, setTestStatus] = useState<"idle" | "testing" | "success" | "error">("idle");

  const handleTest = async () => {
    if (!value.trim()) {
      setTestStatus("error");
      setTimeout(() => setTestStatus("idle"), 3000);
      return;
    }
    setTestStatus("testing");
    try {
      await invoke("test_api_key", { provider, key: value });
      setTestStatus("success");
    } catch {
      setTestStatus("error");
    }
    setTimeout(() => setTestStatus("idle"), 3000);
  };

  return (
    <div className="api-key-field">
      <div className="api-key-field__header">
        <span className="settings-tab__label">{label}</span>
        <span className="settings-tab__description">{description}</span>
      </div>
      <div className="api-key-field__input-row">
        <div className="api-key-field__input-wrapper">
          <input
            type={visible ? "text" : "password"}
            className="api-key-field__input"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder="sk-..."
            spellCheck={false}
          />
          <button
            className="api-key-field__toggle"
            onClick={() => setVisible(!visible)}
            aria-label={visible ? "Hide key" : "Show key"}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
              {visible ? (
                <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24M1 1l22 22" />
              ) : (
                <>
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8Z" />
                  <circle cx="12" cy="12" r="3" />
                </>
              )}
            </svg>
          </button>
        </div>
        <button
          className="settings-tab__btn-secondary"
          onClick={handleTest}
          disabled={testStatus === "testing"}
        >
          {testStatus === "testing" ? (
            <span className="api-key-field__status">...</span>
          ) : testStatus === "success" ? (
            <span className="api-key-field__status api-key-field__status--success">✓</span>
          ) : testStatus === "error" ? (
            <span className="api-key-field__status api-key-field__status--error">✗</span>
          ) : (
            "Test"
          )}
        </button>
      </div>
    </div>
  );
};

const KEY_GROQ_ASR = "talx:key-groq-asr";
const KEY_SONIOX = "talx:key-soniox";
const KEY_GROQ_LLM = "talx:key-groq-llm";
const KEY_OPENAI_LLM = "talx:key-openai-llm";

export const ApiKeysTab = () => {
  const [groqKey, setGroqKey] = useState(() => localStorage.getItem(KEY_GROQ_ASR) ?? "");
  const [sonioxKey, setSonioxKey] = useState(() => localStorage.getItem(KEY_SONIOX) ?? "");
  const [llmGroqKey, setLlmGroqKey] = useState(() => localStorage.getItem(KEY_GROQ_LLM) ?? "");
  const [llmOpenaiKey, setLlmOpenaiKey] = useState(() => localStorage.getItem(KEY_OPENAI_LLM) ?? "");

  const handleGroqKeyChange = (value: string) => {
    setGroqKey(value);
    localStorage.setItem(KEY_GROQ_ASR, value);
    invoke("set_api_key", { provider: "groq", key: value }).catch((err: unknown) => {
      console.error("Failed to set Groq key:", err);
    });
  };

  const handleSonioxKeyChange = (value: string) => {
    setSonioxKey(value);
    localStorage.setItem(KEY_SONIOX, value);
    invoke("set_api_key", { provider: "soniox", key: value }).catch((err: unknown) => {
      console.error("Failed to set Soniox key:", err);
    });
  };

  const handleLlmGroqKeyChange = (value: string) => {
    setLlmGroqKey(value);
    localStorage.setItem(KEY_GROQ_LLM, value);
    invoke("set_api_key", { provider: "llm_groq", key: value }).catch((err: unknown) => {
      console.error("Failed to set Groq LLM key:", err);
    });
  };

  const handleLlmOpenaiKeyChange = (value: string) => {
    setLlmOpenaiKey(value);
    localStorage.setItem(KEY_OPENAI_LLM, value);
    invoke("set_api_key", { provider: "llm_openai", key: value }).catch((err: unknown) => {
      console.error("Failed to set OpenAI key:", err);
    });
  };

  return (
    <div className="settings-tab">
      <h2 className="settings-tab__title">API Keys</h2>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Transcription</h3>
        <ApiKeyField
          label="Groq API Key"
          description="Free speech-to-text via Whisper (groq.com)"
          value={groqKey}
          onChange={handleGroqKeyChange}
          provider="groq"
        />
        <ApiKeyField
          label="Soniox API Key"
          description="Paid real-time streaming transcription (soniox.com)"
          value={sonioxKey}
          onChange={handleSonioxKeyChange}
          provider="soniox"
        />
      </section>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Polishing</h3>
        <ApiKeyField
          label="Groq LLM Key"
          description="Free transcript polishing via Llama 3.3 (groq.com)"
          value={llmGroqKey}
          onChange={handleLlmGroqKeyChange}
          provider="llm_groq"
        />
        <ApiKeyField
          label="OpenAI Key"
          description="Paid transcript polishing via GPT-4o mini (openai.com)"
          value={llmOpenaiKey}
          onChange={handleLlmOpenaiKeyChange}
          provider="llm_openai"
        />
      </section>
    </div>
  );
};

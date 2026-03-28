import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ApiKeyFieldProps {
  label: string;
  description: string;
  value: string;
  onChange: (value: string) => void;
}

const ApiKeyField = ({ label, description, value, onChange }: ApiKeyFieldProps) => {
  const [visible, setVisible] = useState(false);
  const [testStatus, setTestStatus] = useState<"idle" | "success" | "error">("idle");

  const handleTest = () => {
    // Stub: simulate a test
    setTestStatus("success");
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
        <button className="settings-tab__btn-secondary" onClick={handleTest}>
          {testStatus === "success" ? (
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

export const ApiKeysTab = () => {
  const [groqKey, setGroqKey] = useState("");
  const [sonioxKey, setSonioxKey] = useState("");
  const [llmKey, setLlmKey] = useState("");

  const handleGroqKeyChange = (value: string) => {
    setGroqKey(value);
    invoke("set_api_key", { provider: "groq", key: value }).catch((err: unknown) => {
      console.error("Failed to set Groq key:", err);
    });
  };

  const handleSonioxKeyChange = (value: string) => {
    setSonioxKey(value);
    invoke("set_api_key", { provider: "soniox", key: value }).catch((err: unknown) => {
      console.error("Failed to set Soniox key:", err);
    });
  };

  const handleLlmKeyChange = (value: string) => {
    setLlmKey(value);
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
        />
        <ApiKeyField
          label="Soniox API Key"
          description="Paid real-time streaming transcription (soniox.com)"
          value={sonioxKey}
          onChange={handleSonioxKeyChange}
        />
      </section>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">LLM</h3>
        <ApiKeyField
          label="OpenAI / Groq API Key"
          description="Used for transcript polishing"
          value={llmKey}
          onChange={handleLlmKeyChange}
        />
      </section>
    </div>
  );
};

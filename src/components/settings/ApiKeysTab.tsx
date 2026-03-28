import { useCallback, useEffect, useRef, useState } from "react";
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

export const ApiKeysTab = () => {
  const [groqKey, setGroqKey] = useState("");
  const [sonioxKey, setSonioxKey] = useState("");
  const [llmGroqKey, setLlmGroqKey] = useState("");
  const [llmOpenaiKey, setLlmOpenaiKey] = useState("");

  // Load keys from backend on mount
  useEffect(() => {
    const loadKeys = async () => {
      const providers: Array<[string, (v: string) => void]> = [
        ["groq", setGroqKey],
        ["soniox", setSonioxKey],
        ["llm_groq", setLlmGroqKey],
        ["llm_openai", setLlmOpenaiKey],
      ];
      for (const [provider, setter] of providers) {
        try {
          const key = await invoke<string | null>("get_api_key", { provider });
          if (key) setter(key);
        } catch (err: unknown) {
          console.error(`Failed to load ${provider} key:`, err);
        }
      }
    };
    loadKeys();
  }, []);

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleKeyChange = useCallback(
    (provider: string, setter: (v: string) => void) => (value: string) => {
      setter(value);
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        invoke("set_api_key", { provider, key: value }).catch((err: unknown) => {
          console.error(`Failed to set ${provider} key:`, err);
        });
      }, 400);
    },
    [],
  );

  return (
    <div className="settings-tab">
      <h2 className="settings-tab__title">API Keys</h2>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Transcription</h3>
        <ApiKeyField
          label="Groq API Key"
          description="Free speech-to-text via Whisper (groq.com)"
          value={groqKey}
          onChange={handleKeyChange("groq", setGroqKey)}
          provider="groq"
        />
        <ApiKeyField
          label="Soniox API Key"
          description="Paid real-time streaming transcription (soniox.com)"
          value={sonioxKey}
          onChange={handleKeyChange("soniox", setSonioxKey)}
          provider="soniox"
        />
      </section>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Polishing</h3>
        <ApiKeyField
          label="Groq LLM Key"
          description="Free transcript polishing via Llama 3.3 (groq.com)"
          value={llmGroqKey}
          onChange={handleKeyChange("llm_groq", setLlmGroqKey)}
          provider="llm_groq"
        />
        <ApiKeyField
          label="OpenAI Key"
          description="Paid transcript polishing via GPT-4o mini (openai.com)"
          value={llmOpenaiKey}
          onChange={handleKeyChange("llm_openai", setLlmOpenaiKey)}
          provider="llm_openai"
        />
      </section>
    </div>
  );
};

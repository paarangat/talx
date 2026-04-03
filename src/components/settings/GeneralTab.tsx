import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

const ASR_PROVIDER_KEY = "talx:asr-provider";
const LLM_PROVIDER_KEY = "talx:llm-provider";
const AUTO_PASTE_KEY = "talx:auto-paste";
const LANGUAGE_KEY = "talx:language";
const HOTKEY_KEY = "talx:hotkey";
const ASR_MODEL_KEY = "talx:asr-model";
const LLM_MODEL_KEY = "talx:llm-model";

export const GeneralTab = () => {
  const [autoPaste, setAutoPaste] = useState(() => {
    const stored = localStorage.getItem(AUTO_PASTE_KEY);
    return stored !== null ? stored === "true" : true;
  });
  const [language, setLanguage] = useState(() => {
    return localStorage.getItem(LANGUAGE_KEY) ?? "hi+en";
  });
  const [hotkey, setHotkey] = useState(() => {
    return localStorage.getItem(HOTKEY_KEY) ?? "alt+space";
  });
  const [recordingHotkey, setRecordingHotkey] = useState(false);
  const [asrProvider, setAsrProvider] = useState(() => {
    return localStorage.getItem(ASR_PROVIDER_KEY) ?? "groq";
  });
  const [llmProvider, setLlmProvider] = useState(() => {
    return localStorage.getItem(LLM_PROVIDER_KEY) ?? "groq";
  });
  const [asrModel, setAsrModel] = useState(() => {
    return localStorage.getItem(ASR_MODEL_KEY) ?? "whisper-large-v3-turbo";
  });
  const [llmModel, setLlmModel] = useState(() => {
    return localStorage.getItem(LLM_MODEL_KEY) ?? "llama-3.3-70b-versatile";
  });
  const [asrModels, setAsrModels] = useState<string[]>([]);
  const [llmModels, setLlmModels] = useState<string[]>([]);
  const [asrModelsLoading, setAsrModelsLoading] = useState(false);
  const [llmModelsLoading, setLlmModelsLoading] = useState(false);
  const [asrCustom, setAsrCustom] = useState(false);
  const [llmCustom, setLlmCustom] = useState(false);

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

  useEffect(() => {
    invoke("set_asr_model", { model: asrModel }).catch((err: unknown) => {
      console.error("Failed to set ASR model:", err);
    });
  }, [asrModel]);

  useEffect(() => {
    invoke("set_llm_model", { model: llmModel }).catch((err: unknown) => {
      console.error("Failed to set LLM model:", err);
    });
  }, [llmModel]);

  const fetchModels = (provider: string, setModels: (m: string[]) => void, setLoading: (b: boolean) => void) => {
    setLoading(true);
    invoke<Array<{ id: string }>>("fetch_models", { provider })
      .then((models) => {
        const ids = models.map((m) => m.id).sort();
        setModels(ids);
      })
      .catch(() => {
        setModels([]);
      })
      .finally(() => {
        setLoading(false);
      });
  };

  useEffect(() => {
    const providerKey = asrProvider === "groq" ? "groq" : "soniox";
    fetchModels(providerKey, setAsrModels, setAsrModelsLoading);
    setAsrCustom(false);
  }, [asrProvider]);

  useEffect(() => {
    const providerKey = llmProvider === "groq" ? "llm_groq" : "llm_openai";
    fetchModels(providerKey, setLlmModels, setLlmModelsLoading);
    setLlmCustom(false);
  }, [llmProvider]);

  const handleProviderChange = (value: string) => {
    setAsrProvider(value);
    localStorage.setItem(ASR_PROVIDER_KEY, value);
    const defaultModel = value === "groq" ? "whisper-large-v3-turbo" : "soniox-v2";
    setAsrModel(defaultModel);
    localStorage.setItem(ASR_MODEL_KEY, defaultModel);
  };

  const handleLlmProviderChange = (value: string) => {
    setLlmProvider(value);
    localStorage.setItem(LLM_PROVIDER_KEY, value);
    const defaultModel = value === "groq" ? "llama-3.3-70b-versatile" : "gpt-4o-mini";
    setLlmModel(defaultModel);
    localStorage.setItem(LLM_MODEL_KEY, defaultModel);
  };

  const handleAsrModelChange = (value: string) => {
    if (value === "__custom__") {
      setAsrCustom(true);
      return;
    }
    setAsrModel(value);
    localStorage.setItem(ASR_MODEL_KEY, value);
  };

  const handleLlmModelChange = (value: string) => {
    if (value === "__custom__") {
      setLlmCustom(true);
      return;
    }
    setLlmModel(value);
    localStorage.setItem(LLM_MODEL_KEY, value);
  };

  const handleAsrCustomSubmit = (value: string) => {
    if (value.trim()) {
      setAsrModel(value.trim());
      localStorage.setItem(ASR_MODEL_KEY, value.trim());
    }
    setAsrCustom(false);
  };

  const handleLlmCustomSubmit = (value: string) => {
    if (value.trim()) {
      setLlmModel(value.trim());
      localStorage.setItem(LLM_MODEL_KEY, value.trim());
    }
    setLlmCustom(false);
  };

  const formatHotkeyDisplay = (hk: string): string => {
    return hk
      .split("+")
      .map((part) => {
        switch (part) {
          case "alt": return "⌥";
          case "ctrl": return "⌃";
          case "shift": return "⇧";
          case "cmd": case "meta": return "⌘";
          case "space": return "Space";
          default: return part.charAt(0).toUpperCase() + part.slice(1);
        }
      })
      .join(" ");
  };

  useEffect(() => {
    if (!recordingHotkey) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        setRecordingHotkey(false);
        return;
      }

      // Ignore lone modifier presses
      if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;

      const parts: string[] = [];
      if (e.ctrlKey) parts.push("ctrl");
      if (e.altKey) parts.push("alt");
      if (e.shiftKey) parts.push("shift");
      if (e.metaKey) parts.push("cmd");

      // Require at least one modifier
      if (parts.length === 0) return;

      const key = e.key === " " ? "space" : e.key.toLowerCase();
      parts.push(key);

      const newHotkey = parts.join("+");
      setHotkey(newHotkey);
      localStorage.setItem(HOTKEY_KEY, newHotkey);
      setRecordingHotkey(false);

      invoke("set_hotkey", { hotkey: newHotkey }).catch((err: unknown) => {
        console.error("Failed to set hotkey:", err);
      });
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [recordingHotkey]);

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
            <span className="settings-tab__hotkey-badge">
              {recordingHotkey ? "Press keys..." : formatHotkeyDisplay(hotkey)}
            </span>
            <button
              className="settings-tab__btn-secondary"
              onClick={() => setRecordingHotkey(!recordingHotkey)}
            >
              {recordingHotkey ? "Cancel" : "Change"}
            </button>
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
        <h3 className="settings-tab__section-header">Speech Model</h3>
        <div className="settings-tab__row">
          <div className="settings-tab__row-info">
            <span className="settings-tab__label">ASR model</span>
            <span className="settings-tab__description">
              {asrModelsLoading ? "Loading models..." : `Current: ${asrModel}`}
            </span>
          </div>
          {asrCustom ? (
            <div className="settings-model-custom">
              <input
                className="settings-model-custom__input"
                type="text"
                defaultValue={asrModel}
                placeholder="Model ID"
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleAsrCustomSubmit(e.currentTarget.value);
                  if (e.key === "Escape") setAsrCustom(false);
                }}
                onBlur={(e) => handleAsrCustomSubmit(e.currentTarget.value)}
              />
            </div>
          ) : (
            <select
              className="settings-select"
              value={asrModels.includes(asrModel) ? asrModel : "__custom_current__"}
              onChange={(e) => handleAsrModelChange(e.target.value)}
              disabled={asrModelsLoading}
            >
              {asrModels.map((m) => (
                <option key={m} value={m}>{m}</option>
              ))}
              {!asrModels.includes(asrModel) && (
                <option value="__custom_current__">{asrModel}</option>
              )}
              <option value="__custom__">Custom...</option>
            </select>
          )}
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
        <h3 className="settings-tab__section-header">Polish Model</h3>
        <div className="settings-tab__row">
          <div className="settings-tab__row-info">
            <span className="settings-tab__label">LLM model</span>
            <span className="settings-tab__description">
              {llmModelsLoading ? "Loading models..." : `Current: ${llmModel}`}
            </span>
          </div>
          {llmCustom ? (
            <div className="settings-model-custom">
              <input
                className="settings-model-custom__input"
                type="text"
                defaultValue={llmModel}
                placeholder="Model ID"
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleLlmCustomSubmit(e.currentTarget.value);
                  if (e.key === "Escape") setLlmCustom(false);
                }}
                onBlur={(e) => handleLlmCustomSubmit(e.currentTarget.value)}
              />
            </div>
          ) : (
            <select
              className="settings-select"
              value={llmModels.includes(llmModel) ? llmModel : "__custom_current__"}
              onChange={(e) => handleLlmModelChange(e.target.value)}
              disabled={llmModelsLoading}
            >
              {llmModels.map((m) => (
                <option key={m} value={m}>{m}</option>
              ))}
              {!llmModels.includes(llmModel) && (
                <option value="__custom_current__">{llmModel}</option>
              )}
              <option value="__custom__">Custom...</option>
            </select>
          )}
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

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Data</h3>
        <div className="settings-tab__row">
          <div className="settings-tab__row-info">
            <span className="settings-tab__label">Transcription history</span>
            <span className="settings-tab__description">
              Delete all saved transcriptions from this device
            </span>
          </div>
          <button
            className="settings-tab__btn-danger"
            onClick={() => {
              invoke("clear_transcriptions").catch((err: unknown) => {
                console.error("Failed to clear transcriptions:", err);
              });
            }}
          >
            Clear History
          </button>
        </div>
      </section>
    </div>
  );
};

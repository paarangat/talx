import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useRecordingControl } from "../../hooks/useRecordingControl";
import {
  ASR_MODEL_KEY,
  ASR_PROVIDER_KEY,
  AUTO_PASTE_KEY,
  DEFAULT_ASR_MODEL,
  DEFAULT_ASR_PROVIDER,
  DEFAULT_HOTKEY,
  DEFAULT_LANGUAGE,
  DEFAULT_LLM_MODEL,
  DEFAULT_LLM_PROVIDER,
  HOTKEY_KEY,
  LANGUAGE_KEY,
  LLM_MODEL_KEY,
  LLM_PROVIDER_KEY,
} from "../../lib/settings";

export const GeneralTab = () => {
  const { isProcessing, isRecording, toggleRecording } = useRecordingControl();
  const [autoPaste, setAutoPaste] = useState(() => {
    const stored = localStorage.getItem(AUTO_PASTE_KEY);
    return stored !== null ? stored === "true" : true;
  });
  const [language, setLanguage] = useState(() => {
    return localStorage.getItem(LANGUAGE_KEY) ?? DEFAULT_LANGUAGE;
  });
  const [hotkey, setHotkey] = useState(() => {
    return localStorage.getItem(HOTKEY_KEY) ?? DEFAULT_HOTKEY;
  });
  const [recordingHotkey, setRecordingHotkey] = useState(false);
  const [asrProvider, setAsrProvider] = useState(() => {
    return localStorage.getItem(ASR_PROVIDER_KEY) ?? DEFAULT_ASR_PROVIDER;
  });
  const [llmProvider, setLlmProvider] = useState(() => {
    return localStorage.getItem(LLM_PROVIDER_KEY) ?? DEFAULT_LLM_PROVIDER;
  });
  const [asrModel, setAsrModel] = useState(() => {
    return localStorage.getItem(ASR_MODEL_KEY) ?? DEFAULT_ASR_MODEL;
  });
  const [llmModel, setLlmModel] = useState(() => {
    return localStorage.getItem(LLM_MODEL_KEY) ?? DEFAULT_LLM_MODEL;
  });
  const [asrModels, setAsrModels] = useState<string[]>([]);
  const [llmModels, setLlmModels] = useState<string[]>([]);
  const [asrModelsLoading, setAsrModelsLoading] = useState(false);
  const [llmModelsLoading, setLlmModelsLoading] = useState(false);
  const [asrCustom, setAsrCustom] = useState(false);
  const [llmCustom, setLlmCustom] = useState(false);

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

  const fetchModels = (
    provider: string,
    setModels: (models: string[]) => void,
    setLoading: (loading: boolean) => void,
  ) => {
    setLoading(true);
    invoke<Array<{ id: string }>>("fetch_models", { provider })
      .then((models) => {
        const ids = models.map((model) => model.id).sort();
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
    const defaultModel = value === "groq" ? DEFAULT_ASR_MODEL : "soniox-v2";
    setAsrModel(defaultModel);
    localStorage.setItem(ASR_MODEL_KEY, defaultModel);
  };

  const handleLlmProviderChange = (value: string) => {
    setLlmProvider(value);
    localStorage.setItem(LLM_PROVIDER_KEY, value);
    const defaultModel = value === "groq" ? DEFAULT_LLM_MODEL : "gpt-4o-mini";
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
      const nextValue = value.trim();
      setAsrModel(nextValue);
      localStorage.setItem(ASR_MODEL_KEY, nextValue);
    }
    setAsrCustom(false);
  };

  const handleLlmCustomSubmit = (value: string) => {
    if (value.trim()) {
      const nextValue = value.trim();
      setLlmModel(nextValue);
      localStorage.setItem(LLM_MODEL_KEY, nextValue);
    }
    setLlmCustom(false);
  };

  const formatHotkeyDisplay = (value: string): string => {
    return value
      .split("+")
      .map((part) => {
        switch (part) {
          case "alt":
            return "Alt";
          case "ctrl":
            return "Ctrl";
          case "shift":
            return "Shift";
          case "cmd":
          case "meta":
            return "Cmd";
          case "space":
            return "Space";
          default:
            return part.charAt(0).toUpperCase() + part.slice(1);
        }
      })
      .join(" ");
  };

  useEffect(() => {
    if (!recordingHotkey) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        setRecordingHotkey(false);
        return;
      }

      if (["Control", "Alt", "Shift", "Meta"].includes(event.key)) {
        return;
      }

      const parts: string[] = [];
      if (event.ctrlKey) parts.push("ctrl");
      if (event.altKey) parts.push("alt");
      if (event.shiftKey) parts.push("shift");
      if (event.metaKey) parts.push("cmd");

      if (parts.length === 0) {
        return;
      }

      const key = event.key === " " ? "space" : event.key.toLowerCase();
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
            <span className="settings-tab__description">Press to start or stop recording</span>
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
        <h3 className="settings-tab__section-header">Microphone</h3>
        <div className="settings-tab__row">
          <div className="settings-tab__row-info">
            <span className="settings-tab__label">Recording test</span>
            <span className="settings-tab__description">
              Start or stop a dictation test directly from settings
            </span>
          </div>
          <div className="settings-tab__row-action">
            <button
              className="settings-tab__btn-secondary"
              onClick={toggleRecording}
              disabled={isProcessing}
            >
              {isProcessing ? "Processing..." : isRecording ? "Stop Recording" : "Start Recording"}
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
                ? "Groq Whisper - free, batch transcription after recording stops"
                : "Soniox - paid, real-time streaming transcription"}
            </span>
          </div>
          <select
            className="settings-select"
            value={asrProvider}
            onChange={(event) => handleProviderChange(event.target.value)}
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
                onKeyDown={(event) => {
                  if (event.key === "Enter") handleAsrCustomSubmit(event.currentTarget.value);
                  if (event.key === "Escape") setAsrCustom(false);
                }}
                onBlur={(event) => handleAsrCustomSubmit(event.currentTarget.value)}
              />
            </div>
          ) : (
            <select
              className="settings-select"
              value={asrModels.includes(asrModel) ? asrModel : "__custom_current__"}
              onChange={(event) => handleAsrModelChange(event.target.value)}
              disabled={asrModelsLoading}
            >
              {asrModels.map((model) => (
                <option key={model} value={model}>
                  {model}
                </option>
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
                ? "Groq LLM - free transcript polishing via Llama 3.3"
                : "OpenAI - paid, high-quality transcript polishing"}
            </span>
          </div>
          <select
            className="settings-select"
            value={llmProvider}
            onChange={(event) => handleLlmProviderChange(event.target.value)}
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
                onKeyDown={(event) => {
                  if (event.key === "Enter") handleLlmCustomSubmit(event.currentTarget.value);
                  if (event.key === "Escape") setLlmCustom(false);
                }}
                onBlur={(event) => handleLlmCustomSubmit(event.currentTarget.value)}
              />
            </div>
          ) : (
            <select
              className="settings-select"
              value={llmModels.includes(llmModel) ? llmModel : "__custom_current__"}
              onChange={(event) => handleLlmModelChange(event.target.value)}
              disabled={llmModelsLoading}
            >
              {llmModels.map((model) => (
                <option key={model} value={model}>
                  {model}
                </option>
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
            onChange={(event) => {
              setLanguage(event.target.value);
              localStorage.setItem(LANGUAGE_KEY, event.target.value);
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

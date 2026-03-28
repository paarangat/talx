import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Provider {
  name: string;
  type: "ASR" | "LLM";
  provider: string;
  configured: boolean;
}

interface ProviderStatusProps {
  onConfigure: () => void;
}

const PROVIDER_DEFS: Array<{ name: string; type: "ASR" | "LLM"; provider: string }> = [
  { name: "Groq Whisper", type: "ASR", provider: "groq" },
  { name: "Soniox", type: "ASR", provider: "soniox" },
  { name: "Groq LLM", type: "LLM", provider: "llm_groq" },
  { name: "OpenAI", type: "LLM", provider: "llm_openai" },
];

export const ProviderStatus = ({ onConfigure }: ProviderStatusProps) => {
  const [providers, setProviders] = useState<Provider[]>(
    PROVIDER_DEFS.map((d) => ({ ...d, configured: false })),
  );

  useEffect(() => {
    const checkProviders = async () => {
      const results = await Promise.all(
        PROVIDER_DEFS.map(async (def) => {
          try {
            const key = await invoke<string | null>("get_api_key", { provider: def.provider });
            return { ...def, configured: key !== null };
          } catch {
            return { ...def, configured: false };
          }
        }),
      );
      setProviders(results);
    };
    checkProviders();
  }, []);

  return (
    <div className="provider-status">
      <div className="provider-status__header">
        <h3 className="home-page__section-title">Providers</h3>
        <button className="provider-status__configure" onClick={onConfigure}>
          Configure
        </button>
      </div>
      <div className="provider-status__list">
        {providers.map((provider) => (
          <div key={provider.name} className="provider-status__item">
            <span
              className={`provider-status__dot ${provider.configured ? "provider-status__dot--active" : ""}`}
            />
            <span className="provider-status__name">{provider.name}</span>
            <span className="provider-status__type">{provider.type}</span>
          </div>
        ))}
      </div>
    </div>
  );
};

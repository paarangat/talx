interface Provider {
  name: string;
  type: "ASR" | "LLM";
  configured: boolean;
}

interface ProviderStatusProps {
  onConfigure: () => void;
}

export const ProviderStatus = ({ onConfigure }: ProviderStatusProps) => {
  // Hardcoded for now — will be dynamic when keychain integration lands
  const providers: Provider[] = [
    { name: "Groq Whisper", type: "ASR", configured: false },
    { name: "Soniox", type: "ASR", configured: false },
    { name: "Groq LLM", type: "LLM", configured: false },
    { name: "OpenAI", type: "LLM", configured: false },
  ];

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

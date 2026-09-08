import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export const DictionaryTab = () => {
  const [terms, setTerms] = useState<string[]>([]);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<string[]>("get_dictionary")
      .then(setTerms)
      .catch((err: unknown) => {
        console.error("Failed to load dictionary:", err);
        setError("Could not load your dictionary");
      });
  }, []);

  // The backend normalizes (trims, de-duplicates, caps) and returns the
  // stored list, so the UI always shows exactly what will be sent to the
  // transcription and polish layers.
  const persist = (next: string[]) => {
    setTerms(next);
    setError(null);
    invoke<string[]>("set_dictionary", { terms: next })
      .then(setTerms)
      .catch((err: unknown) => {
        console.error("Failed to save dictionary:", err);
        setError("Could not save your dictionary");
      });
  };

  const addTerm = () => {
    const value = draft.trim();
    if (!value) {
      return;
    }
    setDraft("");
    persist([...terms, value]);
  };

  const removeTerm = (index: number) => {
    persist(terms.filter((_, i) => i !== index));
  };

  return (
    <div className="settings-tab">

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Known terms</h3>
        <p className="dictionary__hint">
          Names, jargon and product terms Talx should spell correctly. Used to bias transcription
          and to lock spellings during polish.
        </p>

        <div className="dictionary__add">
          <input
            className="dictionary__input"
            type="text"
            value={draft}
            placeholder="Add a name or term"
            aria-label="Add a name or term"
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                addTerm();
              }
            }}
          />
          <button
            className="settings-tab__btn-secondary"
            onClick={addTerm}
            disabled={!draft.trim()}
          >
            Add
          </button>
        </div>

        {error && (
          <span className="settings-tab__inline-error" role="alert">
            {error}
          </span>
        )}

        {terms.length === 0 ? (
          <p className="dictionary__empty">
            No terms yet. Add a colleague&apos;s name, a company, or a product term to stop Talx
            mangling it.
          </p>
        ) : (
          <ul className="dictionary__list">
            {terms.map((term, index) => (
              <li className="dictionary__item" key={`${term}-${index}`}>
                <span className="dictionary__term">{term}</span>
                <button
                  className="dictionary__remove"
                  onClick={() => removeTerm(index)}
                  aria-label={`Remove ${term}`}
                  title={`Remove ${term}`}
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
};

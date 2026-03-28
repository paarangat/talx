import { useState } from "react";

export const GeneralTab = () => {
  const [autoPaste, setAutoPaste] = useState(true);
  const [language, setLanguage] = useState("hi+en");

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
            onClick={() => setAutoPaste(!autoPaste)}
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
            onChange={(e) => setLanguage(e.target.value)}
          >
            <option value="en">English</option>
            <option value="hi+en">Hindi + English</option>
          </select>
        </div>
      </section>
    </div>
  );
};

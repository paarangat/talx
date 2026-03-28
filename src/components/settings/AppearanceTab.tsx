import { useState } from "react";

const positions = [
  "top-left", "top-center", "top-right",
  "middle-left", "middle-center", "middle-right",
  "bottom-left", "bottom-center", "bottom-right",
] as const;

export const AppearanceTab = () => {
  const [activePosition, setActivePosition] = useState("top-right");

  return (
    <div className="settings-tab">
      <h2 className="settings-tab__title">Appearance</h2>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Theme</h3>
        <div className="settings-tab__row">
          <div className="theme-option theme-option--active">
            <div className="theme-option__swatch theme-option__swatch--light" />
            <span className="theme-option__label">Light</span>
          </div>
        </div>
      </section>

      <section className="settings-tab__section">
        <h3 className="settings-tab__section-header">Widget Position</h3>
        <span className="settings-tab__description">
          Choose where the widget appears on screen
        </span>
        <div className="position-grid">
          {positions.map((pos) => (
            <button
              key={pos}
              className={`position-grid__cell ${activePosition === pos ? "position-grid__cell--active" : ""}`}
              onClick={() => setActivePosition(pos)}
              aria-label={pos}
            />
          ))}
        </div>
      </section>
    </div>
  );
};

import { useState } from "react";
import { GeneralTab } from "./components/settings/GeneralTab";
import { ApiKeysTab } from "./components/settings/ApiKeysTab";
import { AppearanceTab } from "./components/settings/AppearanceTab";
import { AboutTab } from "./components/settings/AboutTab";

type SettingsTab = "general" | "apikeys" | "appearance" | "about";

const tabs: Array<{ id: SettingsTab; label: string }> = [
  { id: "general", label: "General" },
  { id: "apikeys", label: "API Keys" },
  { id: "appearance", label: "Appearance" },
  { id: "about", label: "About" },
];

export const Settings = () => {
  const [activeTab, setActiveTab] = useState<SettingsTab>("general");

  const renderTab = () => {
    switch (activeTab) {
      case "general":
        return <GeneralTab />;
      case "apikeys":
        return <ApiKeysTab />;
      case "appearance":
        return <AppearanceTab />;
      case "about":
        return <AboutTab />;
    }
  };

  return (
    <div className="settings-embedded">
      <nav className="settings-embedded__tabs">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            className={`settings-embedded__tab ${activeTab === tab.id ? "settings-embedded__tab--active" : ""}`}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </nav>
      <div className="settings-embedded__content">{renderTab()}</div>
    </div>
  );
};

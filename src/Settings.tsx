import { useState } from "react";
import { Sidebar } from "./components/settings/Sidebar";
import { GeneralTab } from "./components/settings/GeneralTab";
import { ApiKeysTab } from "./components/settings/ApiKeysTab";
import { AppearanceTab } from "./components/settings/AppearanceTab";
import { AboutTab } from "./components/settings/AboutTab";

type SettingsTab = "general" | "apikeys" | "appearance" | "about";

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
    <div className="settings">
      <div className="settings__grain" />
      <Sidebar activeTab={activeTab} onTabChange={setActiveTab} />
      <main className="settings__content">{renderTab()}</main>
    </div>
  );
};

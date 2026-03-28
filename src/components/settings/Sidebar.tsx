type SettingsTab = "general" | "apikeys" | "appearance" | "about";

interface SidebarProps {
  activeTab: SettingsTab;
  onTabChange: (tab: SettingsTab) => void;
}

const tabs: Array<{ id: SettingsTab; label: string; icon: string }> = [
  {
    id: "general",
    label: "General",
    icon: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm0 2a5 5 0 1 0 0-10 5 5 0 0 0 0 10ZM6.343 6.343l-1.414-1.414m13.142 0-1.414 1.414M6.343 17.657l-1.414 1.414m13.142 0-1.414-1.414M4 12H2m20 0h-2M12 4V2m0 20v-2",
  },
  {
    id: "apikeys",
    label: "API Keys",
    icon: "M15 7a2 2 0 1 0 0-4 2 2 0 0 0 0 4Zm-2.414.586L3 17.172V21h3.828l9.586-9.586M15 7l2.5-2.5",
  },
  {
    id: "appearance",
    label: "Appearance",
    icon: "M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z",
  },
  {
    id: "about",
    label: "About",
    icon: "M12 16v-4m0-4h.01M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0Z",
  },
];

export const Sidebar = ({ activeTab, onTabChange }: SidebarProps) => {
  return (
    <nav className="settings__sidebar">
      <div className="settings__sidebar-header">
        <span className="settings__sidebar-title">Settings</span>
      </div>
      <ul className="settings__nav">
        {tabs.map((tab) => (
          <li key={tab.id}>
            <button
              className={`settings__nav-item ${activeTab === tab.id ? "settings__nav-item--active" : ""}`}
              onClick={() => onTabChange(tab.id)}
            >
              <svg
                className="settings__nav-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d={tab.icon} />
              </svg>
              <span>{tab.label}</span>
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
};

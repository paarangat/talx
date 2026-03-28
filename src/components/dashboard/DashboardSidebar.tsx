import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

type DashboardPage = "home" | "settings";

interface DashboardSidebarProps {
  activePage: DashboardPage;
  onPageChange: (page: DashboardPage) => void;
}

const pages: Array<{ id: DashboardPage; label: string; icon: string }> = [
  {
    id: "home",
    label: "Home",
    icon: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-4 0a1 1 0 01-1-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 01-1 1h-2Z",
  },
  {
    id: "settings",
    label: "Settings",
    icon: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm0 2a5 5 0 1 0 0-10 5 5 0 0 0 0 10ZM6.343 6.343l-1.414-1.414m13.142 0-1.414 1.414M6.343 17.657l-1.414 1.414m13.142 0-1.414-1.414M4 12H2m20 0h-2M12 4V2m0 20v-2",
  },
];

export const DashboardSidebar = ({
  activePage,
  onPageChange,
}: DashboardSidebarProps) => {
  useEffect(() => {
    const unlisten = listen<string>("navigate-section", (event) => {
      const section = event.payload;
      if (section === "home" || section === "settings") {
        onPageChange(section);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [onPageChange]);

  return (
    <nav className="dashboard__sidebar">
      <div className="dashboard__sidebar-header">
        <span className="dashboard__sidebar-title">Talx</span>
      </div>
      <ul className="dashboard__nav">
        {pages.map((page) => (
          <li key={page.id}>
            <button
              className={`dashboard__nav-item ${activePage === page.id ? "dashboard__nav-item--active" : ""}`}
              onClick={() => onPageChange(page.id)}
            >
              <svg
                className="dashboard__nav-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d={page.icon} />
              </svg>
              <span>{page.label}</span>
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
};

import { useEffect, useState } from "react";
import { DashboardSidebar } from "./components/dashboard/DashboardSidebar";
import { HomePage } from "./components/dashboard/HomePage";
import { Settings } from "./Settings";
import { subscribeToUiErrors } from "./lib/uiErrors";

type DashboardPage = "home" | "settings";

interface DashboardProps {
  initialSection: DashboardPage;
}

export const Dashboard = ({ initialSection }: DashboardProps) => {
  const [activePage, setActivePage] = useState<DashboardPage>(initialSection);
  const [startupErrors, setStartupErrors] = useState<string[]>([]);

  useEffect(() => {
    return subscribeToUiErrors((messages) => {
      setStartupErrors(messages);
    });
  }, []);

  return (
    <div className="dashboard">
      <div className="dashboard__grain" />
      <DashboardSidebar activePage={activePage} onPageChange={setActivePage} />
      <main className="dashboard__content">
        {startupErrors.length > 0 && (
          <div className="dashboard__notice dashboard__notice--error" role="alert">
            {startupErrors.map((message) => (
              <p key={message}>{message}</p>
            ))}
          </div>
        )}
        {activePage === "home" && <HomePage onNavigate={setActivePage} />}
        {activePage === "settings" && <Settings />}
      </main>
    </div>
  );
};

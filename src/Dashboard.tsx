import { useState } from "react";
import { DashboardSidebar } from "./components/dashboard/DashboardSidebar";
import { HomePage } from "./components/dashboard/HomePage";
import { Settings } from "./Settings";

type DashboardPage = "home" | "settings";

interface DashboardProps {
  initialSection: DashboardPage;
}

export const Dashboard = ({ initialSection }: DashboardProps) => {
  const [activePage, setActivePage] = useState<DashboardPage>(initialSection);

  return (
    <div className="dashboard">
      <div className="dashboard__grain" />
      <DashboardSidebar activePage={activePage} onPageChange={setActivePage} />
      <main className="dashboard__content">
        {activePage === "home" && <HomePage />}
        {activePage === "settings" && <Settings />}
      </main>
    </div>
  );
};

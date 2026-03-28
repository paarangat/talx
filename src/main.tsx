import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { Dashboard } from "./Dashboard";
import "./styles/tokens.css";
import "./styles/animations.css";
import "./styles/App.css";
import "./styles/settings.css";
import "./styles/dashboard.css";

const params = new URLSearchParams(window.location.search);
const windowType = params.get("window");
const initialSection = params.get("section") || "home";

const rootComponent =
  windowType === "dashboard" ? (
    <Dashboard initialSection={initialSection as "home" | "settings"} />
  ) : (
    <App />
  );

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{rootComponent}</React.StrictMode>,
);

import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { App } from "./App";
import { Settings } from "./Settings";
import "./styles/tokens.css";
import "./styles/animations.css";
import "./styles/App.css";
import "./styles/settings.css";

const windowLabel = getCurrentWindow().label;
const isSettings = windowLabel === "settings";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isSettings ? <Settings /> : <App />}
  </React.StrictMode>,
);

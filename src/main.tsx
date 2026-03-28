import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { App } from "./App";
import { Dashboard } from "./Dashboard";
import "./styles/tokens.css";
import "./styles/animations.css";
import "./styles/App.css";
import "./styles/settings.css";
import "./styles/dashboard.css";

// Sync saved API keys to Rust backend on startup
const savedTranscriptionKey = localStorage.getItem("talx:transcription-key") ?? "";
if (savedTranscriptionKey) {
  invoke("set_api_key", { provider: "groq", key: savedTranscriptionKey }).catch(() => {});
  invoke("set_api_key", { provider: "soniox", key: savedTranscriptionKey }).catch(() => {});
}
const savedAsrProvider = localStorage.getItem("talx:asr-provider") ?? "groq";
invoke("set_asr_provider", { provider: savedAsrProvider }).catch(() => {});

const params = new URLSearchParams(window.location.search);
const windowType = params.get("window");
const rawSection = params.get("section") || "home";
const initialSection: "home" | "settings" = rawSection === "settings" ? "settings" : "home";

const rootComponent =
  windowType === "dashboard" ? (
    <Dashboard initialSection={initialSection} />
  ) : (
    <App />
  );

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{rootComponent}</React.StrictMode>,
);

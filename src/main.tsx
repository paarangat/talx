import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { App } from "./App";
import { Dashboard } from "./Dashboard";
import {
  ASR_MODEL_KEY,
  ASR_PROVIDER_KEY,
  DEFAULT_ASR_MODEL,
  DEFAULT_ASR_PROVIDER,
  DEFAULT_LLM_MODEL,
  DEFAULT_LLM_PROVIDER,
  LLM_MODEL_KEY,
  LLM_PROVIDER_KEY,
} from "./lib/settings";
import { getErrorMessage, reportUiError } from "./lib/uiErrors";
import "./styles/tokens.css";
import "./styles/animations.css";
import "./styles/App.css";
import "./styles/settings.css";
import "./styles/dashboard.css";

const syncStartupSetting = (
  command: string,
  payload: Record<string, string>,
  label: string,
) => {
  invoke(command, payload).catch((err: unknown) => {
    const message = `Failed to sync ${label}: ${getErrorMessage(err, "Unknown error.")}`;
    console.error(message, err);
    reportUiError(message);
  });
};

const savedAsrProvider = localStorage.getItem(ASR_PROVIDER_KEY) ?? DEFAULT_ASR_PROVIDER;
syncStartupSetting("set_asr_provider", { provider: savedAsrProvider }, "speech provider");

const savedAsrModel = localStorage.getItem(ASR_MODEL_KEY) ?? DEFAULT_ASR_MODEL;
syncStartupSetting("set_asr_model", { model: savedAsrModel }, "speech model");

const savedLlmProvider = localStorage.getItem(LLM_PROVIDER_KEY) ?? DEFAULT_LLM_PROVIDER;
syncStartupSetting("set_llm_provider", { provider: savedLlmProvider }, "polish provider");

const savedLlmModel = localStorage.getItem(LLM_MODEL_KEY) ?? DEFAULT_LLM_MODEL;
syncStartupSetting("set_llm_model", { model: savedLlmModel }, "polish model");

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

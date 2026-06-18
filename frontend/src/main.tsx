import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { AppErrorBoundary } from "./AppErrorBoundary";
import { DesktopHudApp } from "./DesktopHudApp";
import "./index.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Root element not found");
}

createRoot(root).render(
  <StrictMode>
    <AppErrorBoundary>{isHudView() ? <DesktopHudApp /> : <App />}</AppErrorBoundary>
  </StrictMode>
);

function isHudView() {
  return new URLSearchParams(window.location.search).get("view") === "hud";
}

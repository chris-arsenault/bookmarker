import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { DesktopHudApp } from "./DesktopHudApp";
import "./index.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Root element not found");
}

createRoot(root).render(<StrictMode>{isHudView() ? <DesktopHudApp /> : <App />}</StrictMode>);

function isHudView() {
  return new URLSearchParams(window.location.search).get("view") === "hud";
}

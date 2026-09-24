import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { attachConsole } from "@tauri-apps/plugin-log";
import App from "./App";
import Settings from "./Settings";
import Strip from "./Strip";

// Forward console.* from every window into the shared log (stdout + the
// OS-standard log dir), so swallowed errors become inspectable.
attachConsole();

// One bundle, routed by window label: panel-N (per-display grid UI),
// settings (config editor), hl-* (target highlight strips).
const label = getCurrentWindow().label;
const Root = label === "settings" ? Settings : label.startsWith("hl-") ? Strip : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);

import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { type ReactNode, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@pokedojo/ui/src/globals.css";

import { MainWindow } from "./MainWindow";
import { CaptureDrawer } from "./overlay/CaptureDrawer";
import { ContinueShield } from "./overlay/ContinueShield";

function rootFor(label: string): ReactNode {
  switch (label) {
    case "capture-drawer":
      return <CaptureDrawer />;
    case "continue-shield":
      return <ContinueShield />;
    default:
      return <MainWindow />;
  }
}

const label = getCurrentWebviewWindow().label;
if (label !== "main") {
  // Overlay windows are transparent; only their content should paint.
  document.documentElement.classList.add("bg-transparent");
  document.body.classList.add("bg-transparent");
}

const rootEl = document.getElementById("root");
if (rootEl === null) {
  throw new Error("#root element not found");
}

createRoot(rootEl).render(<StrictMode>{rootFor(label)}</StrictMode>);

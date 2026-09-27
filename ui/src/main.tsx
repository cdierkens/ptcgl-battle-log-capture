import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@pokedojo/ui/src/globals.css";

import { App } from "./App";

const rootEl = document.getElementById("root");
if (rootEl === null) {
  // eslint-disable-next-line no-restricted-syntax -- Tauri bootstrap fails fast if the mount node is absent
  throw new Error("#root element not found");
}

createRoot(rootEl).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
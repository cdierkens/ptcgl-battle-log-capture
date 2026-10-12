import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri expects a fixed port and must keep the Rust build out of the watcher.
export default defineConfig({
  build: { target: "es2022" },
  clearScreen: false,
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Settings {
  player_name: string;
  output_dir: string;
}

interface CaptureStatus {
  state: string;
  message: string;
  detail: string | null;
  ts: number;
}

const form = document.getElementById("settings-form") as HTMLFormElement;
const playerName = document.getElementById("player-name") as HTMLInputElement;
const outputDir = document.getElementById("output-dir") as HTMLInputElement;
const status = document.getElementById("status") as HTMLParagraphElement;
const log = document.getElementById("capture-log");
if (log === null) {
  throw new Error("#capture-log element not found");
}

function setStatus(message: string): void {
  status.textContent = message;
}

async function loadSettings(): Promise<void> {
  try {
    const s = await invoke<Settings>("get_settings");
    playerName.value = s.player_name;
    outputDir.value = s.output_dir;
  } catch (err) {
    setStatus(`Failed to load settings: ${err}`);
  }
}

async function initLog(): Promise<void> {
  await listen<CaptureStatus>("capture-status", (event) => {
    const line = document.createElement("div");
    const time = new Date(event.payload.ts).toLocaleTimeString();
    const detail =
      event.payload.detail === null || event.payload.detail === ""
        ? ""
        : ` — ${event.payload.detail}`;
    line.textContent = `${time} [${event.payload.state}] ${event.payload.message}${detail}`;
    log.prepend(line);
    while (log.children.length > 50) {
      log.removeChild(log.lastChild);
    }
  });
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const settings: Settings = {
    player_name: playerName.value,
    output_dir: outputDir.value,
  };
  try {
    await invoke("save_settings", { s: settings });
    setStatus("Settings saved.");
  } catch (err) {
    setStatus(`Failed to save settings: ${err}`);
  }
});

void loadSettings();
void initLog();
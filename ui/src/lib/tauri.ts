import { invoke } from "@tauri-apps/api/core";

export interface CaptureStatusEvent {
  detail: null | string;
  message: string;
  state: string;
  ts: number;
}

export interface LogSummary {
  createdMs: number;
  fileName: string;
  sizeBytes: number;
}

export interface Settings {
  output_dir: string;
  player_name: string;
}

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (s: Settings) => invoke<void>("save_settings", { s });
export const listLogs = () => invoke<LogSummary[]>("list_logs");
export const readLog = (fileName: string) => invoke<string>("read_log", { fileName });
export const getCaptureRunning = () => invoke<boolean>("get_capture_running");
export const setCaptureRunning = (running: boolean) => invoke<void>("set_capture_running", { running });
export const getCaptureState = () => invoke<string>("get_capture_state");

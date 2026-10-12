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

export interface OverlayStateEvent {
  autosaved: boolean;
  path: null | string;
  state: string;
}

export interface Settings {
  clipboard_autosave: boolean;
  output_dir: string;
  player_name: string;
  prompt_after_match: boolean;
}

export type SubmitOutcome = { kind: "alreadySaved" } | { kind: "saved"; path: string };

export const OVERLAY_STATE_EVENT = "overlay://state";
export const SETTINGS_CHANGED_EVENT = "settings://changed";

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (s: Settings) => invoke<void>("save_settings", { s });
export const listLogs = () => invoke<LogSummary[]>("list_logs");
export const readLog = (fileName: string) => invoke<string>("read_log", { fileName });

export const getCaptureState = () => invoke<string>("get_capture_state");
export const submitBattleLog = (text: string) => invoke<SubmitOutcome>("submit_battle_log", { text });
export type ClipboardAccess = "allowed" | "ask" | "default" | "denied";

export const dismissShield = () => invoke<void>("dismiss_shield");
export const getClipboardAccess = () => invoke<ClipboardAccess>("get_clipboard_access");
export const requestClipboardAccess = () => invoke<ClipboardAccess>("request_clipboard_access");
export const deleteLog = (fileName: string) => invoke<void>("delete_log", { fileName });
export const undoLastSave = (path: string) => invoke<void>("undo_last_save", { path });
export const revealLog = (fileName: string) => invoke<void>("reveal_log", { fileName });
export const pickOutputDir = () => invoke<null | string>("pick_output_dir");
export const getDiagnostics = () => invoke<string>("get_diagnostics");
export const isFirstRun = () => invoke<boolean>("is_first_run");
export const setDrawerOpen = (open: boolean) => invoke<void>("set_drawer_open", { open });
export const openLogsFolder = () => invoke<void>("open_logs_folder");
export const openScreenRecordingSettings = () => invoke<void>("open_screen_recording_settings");
export const relaunchApp = () => invoke<void>("relaunch_app");
export const CAPTURE_STATUS_EVENT = "capture-status";

import type { ImportPreview } from "./tauri";

/** A required token must be pasted before the import can be saved. */
export function canConfirm(preview: ImportPreview, token: string): boolean {
  return preview.token !== "required" || token.trim() !== "";
}

export function hostOf(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return url;
  }
}

/** `null` keeps the current token (or uses the config's own). */
export function tokenToSend(preview: ImportPreview, token: string): null | string {
  return preview.token === "embedded" || token.trim() === "" ? null : token.trim();
}

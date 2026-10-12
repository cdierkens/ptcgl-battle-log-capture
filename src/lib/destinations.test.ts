import { describe, expect, it } from "vitest";

import type { ImportPreview } from "./tauri";

import { canConfirm, hostOf, tokenToSend } from "./destinations";

function preview(token: ImportPreview["token"]): ImportPreview {
  return { name: "PokeDojo", replaces: null, token, tokenUrl: null, url: "https://pokedojo.pro/api/battle-logs" };
}

describe("hostOf", () => {
  it("shows only the host", () => {
    expect(hostOf("https://pokedojo.pro/api/battle-logs")).toBe("pokedojo.pro");
  });

  it("falls back to the raw text", () => {
    expect(hostOf("nope")).toBe("nope");
  });
});

describe("canConfirm", () => {
  it("needs a token only when the config requires one", () => {
    expect(canConfirm(preview("required"), "  ")).toBe(false);
    expect(canConfirm(preview("required"), "pdj_1")).toBe(true);
    expect(canConfirm(preview("optional"), "")).toBe(true);
    expect(canConfirm(preview("embedded"), "")).toBe(true);
  });
});

describe("tokenToSend", () => {
  it("sends a trimmed pasted token", () => {
    expect(tokenToSend(preview("required"), " pdj_1 ")).toBe("pdj_1");
  });

  it("sends nothing when blank or the config has its own", () => {
    expect(tokenToSend(preview("optional"), " ")).toBeNull();
    expect(tokenToSend(preview("embedded"), "ignored")).toBeNull();
  });
});

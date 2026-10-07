import { describe, expect, it } from "vitest";

import { actionForOutcome, ALREADY_SAVED_MESSAGE, initialPaste, isBlank, pasteReducer } from "./paste-box";

const filled = pasteReducer(initialPaste, { text: "Setup", type: "edit" });

describe("pasteReducer", () => {
  it("edits the text", () => {
    expect(filled.text).toBe("Setup");
  });
  it("refuses to submit blank text", () => {
    expect(pasteReducer(initialPaste, { type: "submit" })).toEqual({
      error: "Paste your Battle Log first.",
      submitting: false,
      text: "",
    });
  });
  it("submits non-blank text", () => {
    expect(pasteReducer(filled, { type: "submit" })).toEqual({ error: null, submitting: true, text: "Setup" });
  });
  it("shows a failure inline and keeps the text", () => {
    const submitting = pasteReducer(filled, { type: "submit" });
    expect(pasteReducer(submitting, { message: "nope", type: "failed" })).toEqual({
      error: "nope",
      submitting: false,
      text: "Setup",
    });
  });
  it("resets after a save", () => {
    expect(pasteReducer(pasteReducer(filled, { type: "submit" }), { type: "saved" })).toEqual(initialPaste);
  });
  it("editing clears the error", () => {
    const errored = pasteReducer(initialPaste, { type: "submit" });
    expect(pasteReducer(errored, { text: "a", type: "edit" }).error).toBeNull();
  });
});

describe("isBlank", () => {
  it("treats whitespace as blank", () => {
    expect(isBlank("  \n")).toBe(true);
    expect(isBlank("Setup")).toBe(false);
  });
});

describe("actionForOutcome", () => {
  it("treats a new save as saved", () => {
    expect(actionForOutcome({ kind: "saved", path: "/x" }, "Results")).toEqual({ type: "saved" });
  });
  it("rejects a re-pasted log while this match is unsaved", () => {
    expect(actionForOutcome({ kind: "alreadySaved" }, "Results")).toEqual({ message: ALREADY_SAVED_MESSAGE, type: "failed" });
    expect(actionForOutcome({ kind: "alreadySaved" }, "InMatch")).toEqual({ message: ALREADY_SAVED_MESSAGE, type: "failed" });
  });
  it("accepts a re-paste once this match is saved", () => {
    expect(actionForOutcome({ kind: "alreadySaved" }, "Saved")).toEqual({ type: "saved" });
  });
});

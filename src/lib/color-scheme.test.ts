import { describe, expect, it } from "vitest";

import { followSystemColorScheme } from "./color-scheme";

function fakes(initiallyDark: boolean) {
  const listeners = new Set<() => void>();
  const classes = new Set<string>();
  const query = {
    addEventListener: (_: "change", listener: () => void) => listeners.add(listener),
    matches: initiallyDark,
    removeEventListener: (_: "change", listener: () => void) => listeners.delete(listener),
  };
  const root = { classList: { toggle: (token: string, force: boolean) => (force ? classes.add(token) : classes.delete(token)) } };
  const change = (dark: boolean) => {
    query.matches = dark;
    listeners.forEach((listener) => listener());
  };
  return { change, classes, listeners, query, root };
}

describe("followSystemColorScheme", () => {
  it("applies the current scheme immediately", () => {
    const { classes, query, root } = fakes(true);
    followSystemColorScheme(root, query);
    expect(classes.has("dark")).toBe(true);
  });

  it("follows OS changes live until unsubscribed", () => {
    const { change, classes, listeners, query, root } = fakes(false);
    const stop = followSystemColorScheme(root, query);
    change(true);
    expect(classes.has("dark")).toBe(true);
    change(false);
    expect(classes.has("dark")).toBe(false);
    stop();
    expect(listeners.size).toBe(0);
  });
});

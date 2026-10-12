---
status: accepted
---

# Uploads are deliveries of saved logs

A battle log is always saved to the log folder first. Sending it anywhere is a separate
delivery of that saved log to a user-configured destination. A destination never
replaces, gates, or delays the local save. A failed delivery leaves the saved log intact,
so it can be retried, and the player never loses a log to a network error or a bad
destination config.

The app ships no hard-coded upload target. PokeDojo, if supported, is a preset
destination like any other.

## Consequences

- The upload spec builds on the existing `LogSink` / `LogStore` seam, which already makes
  saving the single entry point. It adds a delivery queue keyed by the saved log.
- A PokeDojo preset needs a machine credential that PokeDojo does not have today. Its
  only identity is an httpOnly, `SameSite=Lax` session cookie (`pokedojo-auth`), minted
  by the server from a Firebase ID token the browser sends to `/auth/session` (`apps/web/app/lib/auth/identity.server.ts`).
  Loaders and actions read no `Authorization` header, so a desktop app has nothing to send.
  The preset therefore depends on PokeDojo shipping an API token or device flow. It is not a
  capture-side task.
- Analysis (`GameSummary`) exists only in TypeScript, through the npm parser. A body
  mapping that uses it must run in a webview, or the parse must be made available to
  Rust. The upload spec decides which, along with keychain-backed secrets and how to
  send cross-origin requests from Tauri.

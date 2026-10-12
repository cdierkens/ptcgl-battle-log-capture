---
status: accepted
---

# Each OS plugs into the capture engine through platform ports

The capture engine depends only on the traits in `src-tauri/src/ports.rs`:
`WindowProvider`, `FrameCapturer`, `TextRecognizer`, `ScreenCaptureAccess`, `LogStore`,
and `OverlayPresenter`, plus the clipboard and focus probes. Each OS supplies one
implementation per port under `platform/`, selected with `cfg(target_os)`, in the same
crate. That shape already exists; we are not adding a plugin system or a separate crate
per platform. Ports are the seam that keeps the engine unit-testable without a live game,
and one crate keeps a single engine.

## Consequences

- Windows adds implementations (`Windows.Media.Ocr`, the Graphics Capture API) next to
  the macOS ones. It also moves the engine's direct `use` of concrete macOS types
  (`VisionTextRecognizer`, `XcapGame`, `SystemClipboard`, `WorkspaceFocusProbe`) behind
  a single per-OS constructor. The extraction leaves that wiring as it is.
- A port observes the game and never drives it (ADR 0001).
- Ports are desktop-shaped: a game *window* and overlay *windows*. Android has neither.
  The Android spike decides whether it fits these ports or needs its own engine
  front-end. The desktop ports should not be bent to fit it in advance.

---
status: accepted
---

# Saved logs live in the home folder; settings live in app config

The default log folder is `<home>/.ptcgl-battle-logs`, resolved through Tauri's
cross-platform `home_dir()`, and the player can move it in Settings. Settings live in the
OS app-config folder, which is keyed by the bundle identifier, e.g.
`~/Library/Application Support/dev.dierkens.ptcgl-tracker/settings.json`. Saved logs are
the player's data. They should outlive reinstalls and identifier changes, and other tools
should find them in the same place on every OS. Settings are app state, and they belong
where the OS expects app state.

## Considered Options

- **Logs in `<Documents>/PTCGL Tracker`:** rejected. It means moving existing logs, and
  macOS "Desktop & Documents in iCloud" can evict them to the cloud.
- **Everything in app data:** rejected. Logs would be hidden, and tied to the identifier.

## Consequences

- The extraction moves `settings.json` to app config without a migration. The only
  pre-extraction install is the author's, who re-enters settings once.
- `home_dir()` stops reading `$HOME` directly. On Windows `$HOME` is usually unset, and the
  old fallback was the current working directory.
- Treat the log folder path as public once binaries ship. Moving it later needs a
  migration.
- Android has no home folder. The Android spike decides where its saved logs go.

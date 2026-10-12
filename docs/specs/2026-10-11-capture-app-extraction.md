# Spec: Extract the capture app into `ptcgl-battle-log-capture`

Status: **Draft — grilling** (2026-10-11)
Supersedes: the "Planned extractions" bullet in the workspace `AGENTS.md` (private repo, vendored UI).
Context: PokeDojo now consumes the npm parser too (`pokedojo.pro` ADR 0014, #488).

## Goal

Move `pokedojo.pro/apps/battle-log-capture` (Tauri 2 app, product name
"PTCGL Tracker") into its own **public, MIT** repo, working exactly as it does
today on macOS. No new features. Later specs build on this repo (see
[Out of scope → follow-up specs](#out-of-scope--follow-up-specs)).

## Decisions

| # | Decision | Why |
|---|---|---|
| D1 | Extraction ships alone; features come in later specs | Small, low-risk move |
| D2 | Public repo `cdierkens/ptcgl-battle-log-capture`, MIT | GitHub Releases become the download page and Tauri updater endpoint for free |
| D3 | Parse with npm `@dierkens.dev/ptcgl-battle-log-parser` (≥1.1.0) | Drop-in: exports `analyzeBattleLog` + `GameSummary`. Removes `@pokedojo/logic`, `@pokedojo/types`, and `effect` |
| D4 | Adopt shadcn/ui during extraction; do not vendor `@pokedojo/ui` | Vendoring would be thrown away by the theming spec |
| D5 | Tailwind **v4** (CSS-first `@theme`) | shadcn's current default; a theme becomes a CSS file. PokeDojo's v3 pin does not apply here |
| D6 | Bundle identifier → `dev.dierkens.ptcgl-tracker` | Must be permanent before public binaries exist. Accept a one-time settings and Screen Recording permission reset |
| D7 | Plain TypeScript + React. No Effect, no PokeDojo conventions | Same reasoning as the parser repo |

## Current state (verified 2026-10-11)

- `ui/` (Vite + React) and `src-tauri/` (Rust). 5 commits of history.
- PokeDojo deps:
  - `@pokedojo/ui` (10 files)
  - `@pokedojo/logic` + `@pokedojo/types` (only `ui/src/lib/match-row.ts` and its test)
  - `@pokedojo/config` (`tailwind.config.ts`, `eslint.config.js`)
- `effect` is used only in `match-row.ts`.
- No upload code on `main`. The upload spec in `pokedojo.pro/docs/specs/` was never built and is stale.
- No CI. macOS only (OCR via macOS Vision).
- Only Rust reference to PokeDojo: a doc comment in `src-tauri/src/gate.rs`.

## Plan

### 1. Create the repo with history — **done 2026-10-11**

- `git subtree split` of `origin/main` at `6fdee890` produced 5 commits, pushed as `main`.
- The history was scanned before going public: no secrets, no build output, and author email `chris@dierkens.dev` only.
- Added LICENSE (MIT), `.gitignore`, `AGENTS.md`, and the committed `.agents/skills/` + `skills-lock.json`.
- **Carry-over:** `pokedojo.pro` has *uncommitted* WIP in `src-tauri` (`image` crate in `Cargo.toml`/`Cargo.lock`, `src/platform/ocr.rs`). It is not in this repo. Port it here or discard it before step 7.

### 2. Flatten to a standalone pnpm project

- Hoist `ui/` to the repo root (Tauri's default layout). There is now only one JS project.
- Root `package.json`: name it `ptcgl-battle-log-capture`, `"private": true` (an app, not an npm package).
- Replace `catalog:` and `workspace:*` versions with pinned versions.
- Inline the ESLint and TS config previously taken from `@pokedojo/config`.

### 3. Parser swap

`match-row.ts`:

```ts
import { analyzeBattleLog, type GameSummary, isOk } from "@dierkens.dev/ptcgl-battle-log-parser";

const result = analyzeBattleLog(raw, { playerName: playerName.trim() });
return isOk(result) ? { opponentName: result.value.opponentName, summary: result.value.summary } : null;
```

Then remove `effect`. Fix the `gate.rs` doc comment to name the npm package.

### 4. shadcn + Tailwind v4

- Run `shadcn init` (Vite, Tailwind v4, CSS variables). The theme tokens live in one `src/styles/theme.css`.
- Add the shadcn equivalents of the components in use. The table maps each one:

  | `@pokedojo/ui` | shadcn |
  |---|---|
  | Button, Badge, Input, Textarea, Switch, Skeleton | same name |
  | Tabs / TabList / Tab / TabPanel | `tabs` (Tabs / TabsList / TabsTrigger / TabsContent) |
  | Menu / MenuItem | `dropdown-menu` |
  | ConfirmDialog | `alert-dialog` |
  | Callout | `alert` |
  | Field | `label` + small local `Field` wrapper (label, hint, error) |
  | Spinner, PageSpinner, EmptyState, CopyButton | small local components composed from shadcn primitives |
  | `cn` | shadcn `lib/utils` |

- Drop `tailwindcss-react-aria-components`, `autoprefixer`, and `tailwind.config.ts`.
- **Parity rule:** every async action still shows a pending state, and empty and error states still render. Visual parity is not required. Behavioural parity is.

### 5. Identifier and naming

- `tauri.conf.json`: set `identifier` to `dev.dierkens.ptcgl-tracker`. Keep `productName` "PTCGL Tracker" and `version` 0.1.0.
- README: document the one-time Screen Recording re-grant for anyone upgrading from a PokeDojo-built copy.

### 6. CI (verify only)

GitHub Actions on PRs and `main`, `macos-latest`:

- `pnpm install --frozen-lockfile`
- `pnpm run verify` (typecheck, lint, test, vite build)
- `cargo check` + `cargo test` in `src-tauri`

Release builds and signing belong to the release spec, not this one.

### 7. Remove from PokeDojo

PR in `pokedojo.pro`: delete `apps/battle-log-capture`, drop it from `pnpm-workspace.yaml`, and point the capture docs/specs/ADRs at the new repo. A short note is enough; don't move them.

### 8. Workspace wiring

- Add a row to `repos.tsv` (`clone`) and to the README/AGENTS layout tables.
- Add a column to the AGENTS conventions table: Tauri + React, no Effect, Tailwind v4, MIT.
- Inherits the root `.envrc` only. The app needs no secrets yet.

## Acceptance

- [ ] Fresh clone: `pnpm install && pnpm tauri dev` captures a match on macOS like today.
- [ ] `pnpm run verify` and `cargo test` pass locally and in CI.
- [ ] No `@pokedojo/*` or `effect` import remains.
- [ ] Match list shows W/L, prizes, and opponent from the npm parser.
- [ ] `pokedojo.pro` builds and tests green after the app is removed.
- [ ] `bootstrap.sh` clones the new repo; `doctor.sh` passes.

## Out of scope → follow-up specs

Ordered by dependency:

1. **Theming:** plug-and-play shadcn themes. Choose a theme by dropping in or selecting a CSS file of tokens, with user-selectable themes in settings. Builds on D4/D5.
2. **Release pipeline:** a tag-triggered GitHub Actions matrix builds signed binaries per platform, publishes to GitHub Releases, and wires up the Tauri updater (`latest.json`). Open questions:
   - Apple notarization (needs a paid Developer ID)
   - Windows code signing (Authenticode / Azure Trusted Signing)
   - a download page or links in the README
3. **Windows support:** the OCR backend abstraction (macOS Vision vs `Windows.Media.Ocr`), screen capture via the Graphics Capture API, the PTCG Live paths on Windows, and calibration differences. Ships via (2).
4. **Upload integrations:** a Postman-style destination config with:
   - method, URL, headers
   - auth (none / bearer / basic / API key)
   - a body mapper from `GameSummary` / `BattleLog` to the request
   - test-send, retry/queue

   PokeDojo becomes one preset, not a hard-coded target. Secrets go in the OS keychain.
5. **Android (exploratory):** Tauri 2 supports Android, but capture differs (MediaProjection screen capture, ML Kit OCR, overlay permission). iOS is ruled out: no cross-app screen capture or overlay. Spike before speccing.

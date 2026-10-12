# Spec: Extract the capture app into `ptcgl-battle-log-capture`

Status: **Approved** (2026-10-11)
Supersedes: the "Planned extractions" bullet in the workspace `AGENTS.md` (private repo, vendored UI).
Context: PokeDojo now consumes the npm parser too (`pokedojo.pro` ADR 0014, #488).
Vocabulary: [`GLOSSARY.md`](../../GLOSSARY.md). Decisions: [`docs/adr/`](../adr/).

## Goal

Move `pokedojo.pro/apps/battle-log-capture` (Tauri 2 app, product name
"PTCGL Tracker") into its own **public, MIT** repo, working exactly as it does
today on macOS. No new features. Later specs build on this repo (see
[Follow-up specs](#follow-up-specs)).

## Decisions

D1–D7 were settled before grilling. D8 onward came out of the grilling and are all
confirmed. Each one names its ADR where it has one.

| # | Decision | Why |
|---|---|---|
| D1 | Extraction ships alone; features come in later specs | Small, low-risk move |
| D2 | Public repo `cdierkens/ptcgl-battle-log-capture`, MIT | GitHub Releases become the download page and Tauri updater endpoint for free |
| D3 | Parse with npm `@dierkens.dev/ptcgl-battle-log-parser` (≥1.1.0) | Drop-in: exports `analyzeBattleLog` + `GameSummary`. Removes `@pokedojo/logic`, `@pokedojo/types`, and `effect` |
| D4 | Adopt shadcn/ui during extraction; do not vendor `@pokedojo/ui` | Vendoring would be thrown away by the theming spec |
| D5 | Tailwind **v4** (CSS-first `@theme`) | shadcn's current default; a theme becomes a CSS file. PokeDojo's v3 pin does not apply here |
| D6 | Bundle identifier → `dev.dierkens.ptcgl-tracker` | Must be permanent before public binaries exist. Saved logs stay in the home folder (ADR 0002). The Screen Recording grant and settings reset once; the author is the only existing user |
| D7 | Plain TypeScript + React. No Effect, no PokeDojo conventions | Same reasoning as the parser repo |
| D8 | Standard Tauri dev loop: add `@tauri-apps/cli`; `devUrl` + `beforeDevCommand`/`beforeBuildCommand`; `frontendDist: ../dist` | Today there is no CLI and no dev server: the UI must be pre-built and run via `cargo run`. `pnpm tauri dev` gives HMR and is what the release spec's `tauri build` needs anyway |
| D9 | Pin toolchains: `rust-toolchain.toml` = 1.86.0; `packageManager` = pnpm 11 (same as the parser repo); Node 24 via `engines` + `.nvmrc`. JS deps at exact versions taken from PokeDojo's catalog at `6fdee890` | Rust 1.86 is implicit today (only a Cargo.toml comment pins `tauri-plugin-opener ~2.5` because of it). Bump toolchains in their own PRs, not the move |
| D10 | Lint/format from scratch, not inlined from `@pokedojo/config`: ESLint = the parser repo's config (`@eslint/js` + `typescript-eslint` recommended) + `eslint-plugin-react-hooks` + `eslint-plugin-perfectionist` (`recommended-natural`). Rust: add `rustfmt.toml` (`max_width = 120`, `use_small_heuristics = "Max"`), one `style:` reformat commit listed in `.git-blame-ignore-revs` | PokeDojo's ESLint config bans `throw`/`try`/`new Promise` in favour of Effect (conflicts with D7) and carries ingest-only rules. `cargo fmt --check` fails today with 141 hunks (29 under the proposed config). Clippy is already clean |
| D11 | Theme contract = shadcn's standard tokens only. The 7 status tones collapse onto shadcn variants, and alerts carry tone icons. Default theme: shadcn `neutral`, light and dark following the OS | ADR 0003. Any off-the-shelf shadcn theme drops in unchanged |
| D12 | `src/components/ui/*` stays exactly as `shadcn add` generates it. App additions are local wrappers (`LoadingButton`, `StatusAlert`, `CopyButton`) | Upgrades are `shadcn add --overwrite` + review; the theming spec never reasons about local edits |
| D13 | Drop the uncommitted `pokedojo.pro` WIP (`image` crate + PNG path in `platform/ocr.rs`). Keep the BMP encoder, and verify it on the author's laptop, which hit the hang | The WIP is a local workaround: "BMP hangs Vision on this machine". Porting it reverts #487's perf win and adds ~60 crates. See [§7](#7-the-bmp--vision-hang) |
| D14 | Rename the crate `battle-log-capture` → `ptcgl-battle-log-capture` (lib `ptcgl_battle_log_capture_lib`) | It names the executable and the updater artefacts. Free now, a migration once binaries ship. Matches the repo |
| D15 | Platform-specific capture stays behind the existing ports; no restructuring in the extraction | ADR 0004 |
| D16 | No game automation, carried over from PokeDojo | ADR 0001 (accepted) |
| D17 | No binaries of any kind (Releases, attachments, shared zips) until the release spec lands | The first public binary must already contain the updater and its public key, or its users are stranded |
| D18 | Saved logs default to `<home>/.ptcgl-battle-logs` via Tauri's `home_dir()`; `settings.json` moves to the OS app-config folder. No migration | ADR 0002. Logs are the player's data; settings are app state. `$HOME` is usually unset on Windows. The author is the only existing user |
| D19 | Uploads deliver already-saved logs to user-configured destinations; nothing is hard-coded | ADR 0005. Nothing changes in the extraction |

## Current state (verified 2026-10-11)

- `ui/` (Vite + React) and `src-tauri/` (Rust). 5 commits of history.
- PokeDojo deps:
  - `@pokedojo/ui`: 10 files, plus `main.tsx` importing `@pokedojo/ui/src/globals.css` (the tokens)
  - `@pokedojo/logic` + `@pokedojo/types`: only `ui/src/lib/match-row.ts` and its test
  - `@pokedojo/config`: `tailwind.config.ts`, `eslint.config.js`, and `tsconfig.json` (`extends ../../../packages/config/base.tsconfig.json`)
- `effect` is used only in `match-row.ts`.
- **No `@tauri-apps/cli` and no dev server.** `tauri.conf.json` has only `frontendDist: ../ui/dist`. Today you build the UI, then run `cargo run` in `src-tauri`.
- Settings and logs live in `~/.ptcgl-battle-logs` (`settings.rs` `base_dir()`), not in the identifier-scoped app-data dir. `home_dir()` reads only `$HOME`.
- Rust toolchain 1.86 is implicit; `.cargo/config.toml` sets `incompatible-rust-versions = "fallback"`.
- Ports already exist (`ports.rs`): `WindowProvider`, `FrameCapturer`, `TextRecognizer`, `LogStore`, `OverlayPresenter`, `ScreenCaptureAccess`. The non-macOS OCR and permission implementations are stubs. `engine.rs` constructs concrete macOS types directly.
- The UI uses PokeDojo tokens `content-primary/secondary/subtle`, `surface-raised`, `status-critical`, `background`. Nothing sets `.dark`.
- `cargo clippy --all-targets`: clean. `cargo fmt --check`: 141 hunks.
- README is stale: monorepo paths and an Accessibility permission the app no longer requests.
- No upload code on `main`. The upload spec in `pokedojo.pro/docs/specs/` was never built and is stale.
- No CI. macOS only (OCR via macOS Vision).
- Only Rust reference to PokeDojo: a doc comment in `src-tauri/src/gate.rs`.

## Plan

### 1. Create the repo with history — **done 2026-10-11**

- `git subtree split` of `origin/main` at `6fdee890` produced 5 commits, pushed as `main`.
- The history was scanned before going public: no secrets, no build output, and author email `chris@dierkens.dev` only.
- Added LICENSE (MIT), `.gitignore`, `AGENTS.md`, and the committed `.agents/skills/` + `skills-lock.json`.
- Added `GLOSSARY.md` and ADRs 0001–0005 (all accepted) during grilling.

### 2. Flatten to a standalone pnpm project

- Hoist `ui/` to the repo root (Tauri's default layout): `index.html`, `src/`, `vite.config.ts`, etc. at root; `src-tauri/` beside them. Use `git mv` so history follows.
- Root `package.json`: name `ptcgl-battle-log-capture`, `"private": true`, `packageManager: pnpm@11.x`, `engines.node: ">=24"`; `.nvmrc` = 24.
- Replace `catalog:` and `workspace:*` with exact versions (D9). Add `@tauri-apps/cli` (v2) as a dev dep.
- Scripts: `dev`, `build`, `tauri`, `typecheck`, `lint`, `test`, and `verify` (= typecheck + lint + test + build).
- `tauri.conf.json` `build`: `devUrl: http://localhost:5173`, `beforeDevCommand: pnpm dev`, `beforeBuildCommand: pnpm build`, `frontendDist: ../dist` (D8).
- `tsconfig.json`: inline the base options (`strict`, `noUnusedLocals`, `noUnusedParameters`, `isolatedModules`, `moduleResolution: Bundler`, `skipLibCheck`).
- `eslint.config.js` per D10 (perfectionist will reorder imports: do that in its own `style:` commit); delete the `no-restricted-syntax` disable in `main.tsx` (the rule no longer exists).
- `rust-toolchain.toml` (1.86.0, `rustfmt` + `clippy` components); `rustfmt.toml`; one reformat commit + `.git-blame-ignore-revs` (D10).
- Rename the crate (D14) and update `examples/bench_region.rs` imports.
- Settings (D18): `settings_path()` → Tauri `app_config_dir()/settings.json`; `base_dir()` → Tauri `home_dir()` + `.ptcgl-battle-logs`. `settings::exists()` keeps driving first run, so the setup flow shows once after the move.

### 3. Parser swap

`match-row.ts`:

```ts
import { analyzeBattleLog, type GameSummary, isOk } from "@dierkens.dev/ptcgl-battle-log-parser";

const result = analyzeBattleLog(raw, { playerName: playerName.trim() });
return isOk(result) ? { opponentName: result.value.opponentName, summary: result.value.summary } : null;
```

`BattleLogAnalysis` in 1.1.0 is `{ locale, opponentName, playerName, summary }`, so this is a drop-in.
Then remove `effect`. Update `match-row.test.ts`'s `GameSummary` import. Fix the `gate.rs` doc comment to name the npm package.

### 4. shadcn + Tailwind v4

- Run `shadcn init` (Vite, Tailwind v4 via `@tailwindcss/vite`, CSS variables, base colour `neutral`). The theme lives in one `src/styles/theme.css` with `:root` and `.dark` blocks of shadcn's standard tokens only (ADR 0003). `src/styles/globals.css` imports it.
- Dark mode: toggle the `.dark` class on `<html>` from `matchMedia("(prefers-color-scheme: dark)")`, and react to changes. The theming spec adds a user override on the same class.
- `src/components/ui/*` is pristine `shadcn add` output (D12). Mapping:

  | `@pokedojo/ui` | Replacement | Notes |
  |---|---|---|
  | Button | `button`; local `LoadingButton` | `isLoading` (6 uses) → `LoadingButton` = Button + `spinner`, disabled while pending |
  | Badge | `badge` | Tones per ADR 0003: `brand`/`positive` → `default`, `info` → `secondary`, `caution` → `outline`, `critical` → `destructive`. W → `default`, L → `destructive`, no result → `secondary` |
  | Callout | `alert`; local `StatusAlert` | Tone → `default`/`destructive` + lucide icon (check, info, triangle, x-circle) |
  | Input, Textarea, Switch, Skeleton | same name | `Switch` `isSelected`/`onChange` → `checked`/`onCheckedChange` |
  | Tabs / TabList / Tab / TabPanel | `tabs` | `defaultSelectedKey` → `defaultValue`; `id` → `value` |
  | Menu / MenuItem | `dropdown-menu` | `onAction` → `onSelect` |
  | ConfirmDialog | `alert-dialog` | |
  | Field | `field` + `label` | |
  | Spinner, PageSpinner | `spinner` | PageSpinner = a centred `spinner` |
  | EmptyState | `empty` | |
  | CopyButton | local `CopyButton` | Button + clipboard write + "copied" state |
  | `cn` | shadcn `lib/utils` | |

- Token classes per ADR 0003: `text-content-primary` → `text-foreground`; `text-content-secondary`/`-subtle` → `text-muted-foreground`; `bg-surface-raised` → `bg-muted`; `bg-status-critical` → `bg-destructive`.
- Keep `main.tsx`'s transparent `html`/`body` for the overlay windows, in both modes.
- Drop `tailwindcss-react-aria-components`, `autoprefixer`, `postcss.config.js`, and `tailwind.config.ts`.
- **Parity rule:** every async action still shows a pending state, and empty and error states still render. Visual parity is not required. Behavioural parity is. Checked by [§8](#8-behavioural-parity-check).

### 5. Identifier and naming

- `tauri.conf.json`: set `identifier` to `dev.dierkens.ptcgl-tracker`. Keep `productName` "PTCGL Tracker" and `version` 0.1.0.
- README rewrite: standalone commands (`pnpm install`, `pnpm tauri dev`, `cargo test`), keep the perf notes, and drop the Accessibility line. Link the glossary and ADR 0001. No upgrade notes: the author is the only existing user.

### 6. CI (verify only)

GitHub Actions on PRs and `main`. Actions are pinned to commit SHAs, and Dependabot (npm, cargo, actions) keeps them current, as in the parser repo.

- `verify` on `macos-latest`: `pnpm install --frozen-lockfile`, `pnpm run verify`, then in `src-tauri`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- `compile` on `windows-latest`: `pnpm build` then `cargo check --all-targets` + `cargo test`. This keeps the non-macOS stubs and the pure core from rotting before the Windows spec. If it is red at extraction time, fix compile errors only; anything behavioural is Windows-spec input.
- Cache pnpm and `~/.cargo` + `src-tauri/target`.

Release builds and signing belong to the release spec, not this one.

### 7. The BMP → Vision hang — **resolved 2026-10-11**

D13 drops the WIP, so the extraction keeps the BMP path (#487).

Probed on the author's laptop with the repo's `VisionTextRecognizer` and no game running:

| Binary | Encoding | First OCR call | Later calls |
|---|---|---|---|
| fresh build | BMP | > 60 s (killed) | — |
| same binary, 2nd run | BMP | 34.5 s | 0.1–1.1 s |
| copy of that binary at a new path | BMP | 78 s | 0.08–0.6 s |
| fresh build with the WIP's PNG path | PNG | 63 s | 0.1–3.5 s |

The "hang" is Vision's cold start: its first request in a **new binary** takes about 35–80 s, regardless of encoding. Every rebuild is a new binary, so during development it looked like BMP was hanging. The WIP's diagnosis was wrong, and porting it would have fixed nothing.

It still matters to users: after every install or update, the engine thread blocks on that first call and results detection is blind for about a minute. That's tracked in #2 (warm Vision up off the engine thread at launch). It's a behaviour change, so it's not part of the extraction.

The WIP in `pokedojo.pro` can be discarded. Step 9 deletes that directory anyway.

### 8. Behavioural parity check

The Rust core is untouched apart from formatting and the rename, so `cargo test` covers it. The UI has unit tests for its `lib/` logic (`capture-drawer`, `capture-status`, `match-row`, `paste-box`, `status-history`). Everything else is checked by hand, on macOS, against a real match, and recorded in the PR:

- [ ] First run: setup flow asks for player name and Screen Recording; relaunch after granting works. `settings.json` lands in app config; logs land in `~/.ptcgl-battle-logs`.
- [ ] Results screen detected → capture drawer opens and Continue shield raises (when "prompt after match" is on).
- [ ] Paste + Save (⌘↵) saves; a non-log paste shows the error; saving the same log again reports already-saved.
- [ ] Clipboard auto-save (opt-in) saves after Export and offers Undo; Undo trashes the file.
- [ ] Shield dismiss works; drawer closes after an auto-save.
- [ ] Match list: W/L badge, prizes, opponent from the npm parser; unparsed logs fall back to date title; empty state; skeleton while loading.
- [ ] Match menu: view raw log, reveal in Finder, delete via confirm dialog (moves to Trash).
- [ ] Settings: player name, save folder picker, toggles persist across relaunch.
- [ ] Help: diagnostic report copies.
- [ ] Every async button shows pending; error alerts render.
- [ ] Light and dark follow the OS setting live, in the main window and both overlays; overlays stay transparent.

### 9. Remove from PokeDojo

PR in `pokedojo.pro`: delete `apps/battle-log-capture`, drop it from `pnpm-workspace.yaml`, and point the capture docs/specs/ADRs at the new repo. A short note is enough; don't move them. The stale upload spec (`2026-10-07-battle-log-upload.md`), its ADR (`0012-battle-log-upload-via-webview.md`) and plan were never merged; they live only on the unpushed local branch `feat/battle-log-upload`. Delete the working-tree copies. Their one live fact, PokeDojo's cookie-only auth, is restated in ADR 0005.

### 10. Workspace wiring

- `repos.tsv` already has the row (`clone`). Add the repo to the README/AGENTS layout tables.
- Add a column to the AGENTS conventions table: Tauri + React, no Effect, Tailwind v4, MIT.
- Inherits the root `.envrc` only. The app needs no secrets yet.

## Acceptance

- [ ] Fresh clone: `pnpm install && pnpm tauri dev` captures a match on macOS like today. This includes the author's laptop (§7).
- [ ] `pnpm run verify`, `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test` pass locally and in CI; the Windows `compile` job is green.
- [ ] No `@pokedojo/*`, `effect`, `tailwind.config.ts`, or PokeDojo token class remains.
- [ ] Match list shows W/L, prizes, and opponent from the npm parser.
- [ ] Parity checklist (§8) is ticked in the PR.
- [ ] `pokedojo.pro` builds and tests green after the app is removed.
- [ ] `bootstrap.sh` clones the new repo; `doctor.sh` passes.

## Follow-up specs

Ordered by dependency and value. This **changes the original order**: the release
pipeline now comes before theming. Theming unblocks nothing else, because the
extraction already fixes its contract (ADR 0003). Releases are what put the app in
anyone's hands, and every later spec ships through them.

| # | Spec | Needs from the extraction | Must decide |
|---|---|---|---|
| 1 | **Release pipeline:** tag-triggered matrix, signed binaries, GitHub Releases, Tauri updater (`latest.json`) | D6 identifier, D14 crate name, D8 `tauri build` path, D17 (nothing published before it), D18 paths are public from the first binary | Apple Developer ID + notarization, or ship unsigned first; updater key custody; Windows signing (Authenticode / Azure Trusted Signing); download links in the README |
| 2 | **Theming:** user-selectable themes, drop-in theme CSS files, dark/system mode | ADR 0003 contract; components use contract tokens only; `.dark` class driven by the OS | User override (system / light / dark); where user themes live (app config per ADR 0002); handling a theme missing tokens or a `.dark` block; whether PokeDojo's palette ships as a bundled theme |
| 3 | **Windows support:** `Windows.Media.Ocr`, Graphics Capture API, PTCG Live paths, calibration | ADR 0004 ports; the Windows `compile` CI job | A per-OS port constructor in place of the engine's direct macOS imports, and renaming the non-macOS `VisionTextRecognizer` stub; results-detection calibration on Windows DPI scaling; the overlay window behaviour that `macOSPrivateApi` currently covers |
| 4 | **Upload integrations:** destinations (method, URL, headers, auth none/bearer/basic/API key), body mapping, test-send, delivery queue + retry; secrets in the OS keychain | ADR 0005 (saved log first; `LogSink` is the single entry point) | Where the body mapping runs (webview vs Rust), given that analysis is TS-only; cross-origin requests from Tauri; the PokeDojo preset is blocked on PokeDojo shipping a machine credential (ADR 0005) |
| 5 | **Android (exploratory spike):** MediaProjection capture, ML Kit OCR, overlay permission. iOS is ruled out (no cross-app capture or overlay) | ADR 0001 (observe, never drive); ADR 0004 (desktop ports are not bent for it) | Whether Android fits the ports or needs its own engine front-end; where saved logs live without a home folder (ADR 0002); the Play policy for overlays and screen capture |

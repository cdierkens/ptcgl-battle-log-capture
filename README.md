# Battle Log Capture

A Tauri desktop app that watches the Pokémon TCG Live window, spots the match
results screen, and saves your battle log to `~/.ptcgl-battle-logs`. It only
reads the screen — nothing is clicked in the game.

## Build the UI first

`src-tauri/tauri.conf.json` points `frontendDist` at `../ui/dist`, which is build output
and is not checked in. A fresh clone (or a fresh git worktree) has no `dist`, and
`cargo build` fails inside `generate_context!` with a message that does not
mention the UI. Always:

```bash
pnpm --filter battle-log-capture-ui build
```

## Run

Developing:

```bash
cd apps/battle-log-capture/src-tauri && cargo run
```

Playing:

```bash
cd apps/battle-log-capture/src-tauri && cargo run --release
```

Release is still the right build to play on, but the gap is much smaller than it
used to be. Measured after the BMP encoder landed (2560x1496 capture):

| | debug | release |
|---|---|---|
| encode a 512x150 region | 2.21 ms | 0.12 ms |
| region OCR | 34.1 ms | 69.5 ms |

Only the encoder is meaningfully profile-sensitive, and in release it costs
0.12 ms — about 2 ms per poll. The OCR itself is macOS Vision (native) and does
not speed up with optimisation; its per-call cost is large and varies with what
is on screen, so debug and release land within each other's noise. Pick release
for the smaller encoder and a faster rebuild-free start, not for a large OCR win.

Vision's cost is also not proportional to crop size: the 512x150 region costs
~33 ms, while a 1.5Mpx band and the 3.8Mpx whole frame both cost ~250 ms. The
curve flattens well before full-frame size, which is why the safety-net sweep
now reads a band — the whole frame's extra pixels were buying nothing.

## macOS permissions

Screen Recording (to read the window) and Accessibility are requested on first
run and are attributed to the terminal you launched from — a terminal restart
may be needed for the prompt to appear.

## Tests

```bash
cd apps/battle-log-capture/src-tauri && cargo test
```

## Bench

`cargo run --release --example bench_region` times the detection OCR against a
live game window. It needs the game open; it prints `no window` otherwise.

It reports the region, the three candidate sweep bands, and the whole frame
back-to-back in one run — **compare within a single run only**. Across runs the
OCR numbers swing by more than 15% with identical code, because Vision's cost
depends on on-screen content. The `encode` lines are deterministic and are the
only figures safe to compare across runs.

# ptcgl-battle-log-capture

Desktop companion ("PTCGL Tracker") that watches Pokémon TCG Live, spots the
match results screen, and saves the battle log. Tauri 2 (Rust) + React. Public,
MIT. Sibling of `@dierkens.dev/ptcgl-battle-log-parser`.

## Status: planning

The code is a `git subtree split` of `pokedojo.pro/apps/battle-log-capture`
and still builds only inside that monorepo (`workspace:*` / `catalog:` deps,
`@pokedojo/*` imports). Do not try to make it build piecemeal. The
extraction is specced in `docs/specs/2026-10-11-capture-app-extraction.md`
and is still being grilled. Implement only once that spec is approved.

## Conventions (target state)

- Plain TypeScript + React. **No Effect**, no `@pokedojo/*`, no PokeDojo rules.
- Parsing: npm `@dierkens.dev/ptcgl-battle-log-parser` (`Result` union, no throw).
- UI: shadcn/ui on Tailwind v4 (CSS-first theme tokens).
- Bundle identifier `dev.dierkens.ptcgl-tracker`. It is permanent once binaries ship.
- Conventional commits.

## Docs

- `docs/specs/`: specs. Draft until grilled, then approved.
- `docs/adr/`: decisions.

## Agent skills

Committed under `.agents/skills/`, sourced via `skills-lock.json`. Update
them deliberately with `npx skills update`, and review the diff.

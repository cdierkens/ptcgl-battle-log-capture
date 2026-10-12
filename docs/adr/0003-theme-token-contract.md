---
status: accepted
---

# The theme contract is shadcn's standard tokens, and nothing else

A theme is one CSS file that sets shadcn/ui's standard Tailwind v4 tokens, for both
`:root` (light) and `.dark`. The tokens are `background`, `foreground`, `card`,
`popover`, `primary`, `secondary`, `muted`, `accent`, `destructive`, `border`, `input`,
`ring`, and `radius`, with their `-foreground` pairs. Components may use only these
tokens, never raw colours or app-specific tokens. Any off-the-shelf shadcn theme
(tweakcn, the shadcn registry) therefore drops in unchanged. That is the point of the
theming spec.

PokeDojo's `content-*`, `surface-*`, `brand-*`, and `status-*` vocabulary is dropped.
`content-primary` → `foreground`; `content-secondary` and `content-subtle` →
`muted-foreground`; `surface-raised` → `muted`; `status-critical` → `destructive`.

## Status tones collapse onto shadcn variants

shadcn has no success, warning, or info colours. The app's seven tones collapse:
`brand` and `positive` → `default` (primary); `info` → `secondary`; `caution` →
`outline`; `critical` → `destructive`; `secondary` and `outline` stay as they are. An alert
carries an icon for its tone (check, info, triangle, x-circle), so the meaning never
rests on colour alone. Match results: W → `default`, L → `destructive`, no result →
`secondary`.

## Considered Options

- **shadcn tokens plus `success`/`warning`/`info` pairs:** rejected. Third-party themes
  don't define them, so every theme would need patching or would fall back to our colours.
- **Map status to `chart-1..5`:** rejected. Each theme assigns those arbitrarily, so
  "success" could render orange.
- **Fixed Tailwind palette colours for status:** rejected. They ignore the theme and can
  clash with it.

## Consequences

- Light and dark both ship at extraction: the `.dark` class follows the OS
  `prefers-color-scheme`. The theming spec adds a user override (system / light / dark)
  on the same class.
- Overlay windows (capture drawer, Continue shield) keep a transparent page background
  in both modes: only their content paints.
- The default theme is shadcn `neutral`. The PokeDojo navy/yellow look can come back as a
  bundled theme.
- Never rename or remove a contract token: that breaks every user theme.

---
status: accepted
---

# No automation of PTCG Live

The app reads PTCG Live's pixels and, if the player opts in, the clipboard. It never sends
input to the game: no clicks, no keystrokes, no scripted Export or Continue. The player
exports the battle log themselves, and the app keeps it. The Pokémon Terms of Use (§3(iii),
with §1(iv) naming PTCG Live) forbid unauthorized third-party software that automates
operation of the game. The earlier auto-pilot (OCR → click Battle Log → Export → Continue)
was also fragile and needed the Accessibility permission.

Carried over from `pokedojo.pro` ADR 0013, which also has the opt-in clipboard auto-save
amendment. Every follow-up spec inherits it, including the Windows and Android ports. A
platform port may observe the game but never drive it.

## Considered Options

- **Opt-in auto-pilot:** rejected. Two flows to maintain, and the ToU-problematic
  automation would still ship.
- **No detection, drawer only:** rejected. Without the Continue shield as a forcing
  function, players still miss logs.

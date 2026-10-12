# PTCGL Tracker

A desktop companion to Pokémon TCG Live (PTCG Live). It notices when a match ends
and helps the player keep that match's battle log, without ever playing the game
for them.

## Language

### The game

**PTCG Live**:
Pokémon TCG Live, the official client the app watches. The app is never part of it.
_Avoid_: PTCGL (except in names), the client, TCGL

**Battle log**:
The per-match play-by-play text PTCG Live puts on the clipboard when the player clicks
Export on the results screen.
_Avoid_: replay, match history, game log

**Results screen**:
The screen PTCG Live shows when a match ends. It holds the Battle Log tab and the
Continue button.
_Avoid_: end screen, post-game screen

**Player name**:
The player's in-game name, exactly as PTCG Live shows it. It tells the player's side of a
battle log from the opponent's.
_Avoid_: username, account, profile

### Capture

**Capture**:
Keeping a match's battle log after the player has exported it themselves. The app may
read the screen and the clipboard. It never clicks, types, or otherwise acts in the game.
_Avoid_: scraping, auto-export, automation

**Results detection**:
Recognising, by reading the screen, that PTCG Live is showing the results screen.
_Avoid_: trigger, hook

**Capture drawer**:
The app's in-game hub. It slides out from the PTCG Live window while the game is active,
and it holds saving this match's log, past matches, and settings.
_Avoid_: capture tab, widget, popup, overlay (on its own)

**Continue shield**:
The overlay that covers the results screen's Continue button until the player saves a
battle log or skips it.
_Avoid_: blocker, lock

**Clipboard auto-save**:
An opt-in setting. While the results screen is up and the game is focused, it saves a
battle log the player has just exported to the clipboard. Each auto-save can be undone.
_Avoid_: clipboard sync, clipboard listener

**Saved log**:
A battle log written to the log folder. It is the app's record of a match and the
source for everything derived from it.
_Avoid_: capture (as a noun), file, entry

**Log folder**:
The folder that saved logs go into. The player chooses it, and it lives outside the app's
own data.
_Avoid_: save folder (UI label only), output dir, app data

**Match**:
One saved log as the player sees it in the list: result, prizes, and opponent, when the
log can be analysed.
_Avoid_: game (ambiguous with PTCG Live), row

### Platform

**Platform port**:
One OS capability the capture engine needs, such as finding the game window, capturing a
frame, recognising text, or presenting overlays. Each OS supplies its own implementation.
_Avoid_: driver, adapter (on its own), backend

### Appearance

**Theme**:
A complete set of colour and shape tokens that restyles the whole app without changing
its behaviour.
_Avoid_: skin, palette, brand

### Upload (planned)

**Destination**:
A user-configured place that saved logs are sent to: a request template, an auth method,
and a body mapping.
_Avoid_: integration, webhook, target, endpoint (on its own)

**Body mapping**:
The rule that turns a saved log, and its analysis, into a destination's request body.
_Avoid_: transformer, template (ambiguous with parser templates)

**Delivery**:
One attempt, and its retries, to send one saved log to one destination.
_Avoid_: upload (as a noun), sync, push

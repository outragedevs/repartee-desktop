# Repartee desktop GUI — iced port (design + plan)

_Date: 2026-06-07_

## Decision recap

Desktop packaging pivoted **from trolley to a native iced GUI**. trolley only
wrapped the ratatui TUI in a window ("polished terminal", wrong *form* for
mIRC/HexChat users) and its Windows path depended on Ghostty's pre-alpha. The
target audience expects **real native widgets**.

- **Keep repartee's MIT backend; write a fresh iced View.** Do **not** fork
  halloy or copy its code — halloy is **GPL-3.0**, repartee is **MIT**. halloy
  is used only as a *design blueprint* (clean-room: study patterns, write our
  own). See `~/.claude/.../memory/desktop-gui-direction.md`.
- repartee's value is its backend (irc-repartee fork, scripting, e2e, storage,
  web-ui, …); halloy's value is its mature iced IRC UI — which proves iced is
  production-capable cross-platform (incl. Windows).

## Architecture

```
repartee-app/                 (Cargo workspace)
├── .            repartee      TUI binary (ratatui/crossterm) — stays Unix-centric
├── web-ui/      repartee-web  Leptos WASM (mobile/remote) — unchanged
└── gui/         repartee-gui  iced native desktop GUI  ← NEW
```

### Why a separate `gui/` crate

The iced GUI depends on **`irc-repartee` directly** plus a couple of repartee's
pure helpers (ported), and **deliberately does not link** ratatui, crossterm,
vt100, portable-pty, the fork/daemon, the Unix-socket session layer, or the PTY
shell. That makes the GUI **Windows-clean by construction** — the terminal-only
Windows blockers (mapped in the recon) simply aren't in its dependency graph:

| Unix/terminal-only (excluded from gui) | Where it lives |
|---|---|
| fork/setsid/daemon | `src/main.rs` |
| session daemon over Unix sockets | `src/session/` |
| embedded PTY shell, vt100 | `src/shell/` |
| ratatui/crossterm rendering | `src/ui/`, `src/app/` |

### Reuse map (verified UI-agnostic in repartee)

- `src/state/`, `src/config/`, `src/irc/` — zero ratatui deps; fully reusable.
- `irc::connect_server()` returns `(IrcHandle, mpsc::Receiver<IrcEvent>)` — the
  exact shape iced wants.
- `src/nick_color.rs`, `src/irc/formatting.rs` — pure logic; **ported** into
  `gui/src/format.rs` (only change: return `iced::Color`, not `ratatui::Color`).

### Eventual `repartee-core` extraction (next phase, not MVP)

To reuse the *full* backend (not just irc-repartee + ports) without dragging in
terminal deps, extract the UI-agnostic modules (`state`, `irc`, `config`, `e2e`,
`storage`, `dcc`, `nick_color`) into a new `repartee-core` library crate that
**both** the TUI binary and `repartee-gui` depend on. The TUI keeps its
ratatui/session/shell modules. This is the clean long-term shape; the MVP
short-circuits it by using `irc-repartee` directly so we get something running
first.

## iced version

- **MVP uses upstream `iced = "0.13"`** (crates.io, stable) for build
  reliability — no git fork, no Zig, runs on macOS/Windows/Linux.
- halloy rides a `squidowl/iced` **0.15-dev** fork for extra patches
  (text selection, focus/IME, richer rendering). Revisit only if a concrete
  feature needs it; document the exact patch we need before adopting a fork.

## IRC ↔ iced bridge (halloy pattern, adapted)

`gui/src/irc_client.rs`: `connect()` returns
`impl Stream<Item = Event>` built with `iced::stream::channel`. Inside the task
we `Client::from_config(...).await`, `identify()`, grab `sender`, and forward
each protocol message as `Event::Message`. The app subscribes via
`Subscription::run(irc_client::connect).map(Message::Irc)`. The command
`Sender` is handed to the app through `Event::Connected(sender, nick)` so
`update()` can send PRIVMSG/commands. Auto-scroll uses
`scrollable::snap_to(.., RelativeOffset::END)` after appends.

## Aesthetic

Layout mirrors halloy: **buffers sidebar · chat log · nick list**, with a topic
header and a bottom input. Theme is a warm-dark palette (ferra-ish), but the
**main font is our bundled FiraCode Nerd Font Mono** (`assets/fonts/`, restored
from the trolley work). Nick colors use repartee's deterministic djb2+HSL.

## `gui/` module layout

| File | Role |
|---|---|
| `main.rs` | iced entry: fonts, theme, window, `run_with` |
| `app.rs` | `App` state, `Message`, `update`, IRC routing, sending |
| `irc_client.rs` | `irc-repartee` → iced `Subscription` bridge |
| `ui.rs` | view: sidebar, chat log, nick list, input |
| `theme.rs` | palette, fonts, per-nick color |
| `format.rs` | ported nick-color + mIRC strip (MIT) |
| `state.rs` | GUI buffers/lines/users (subset of `src/state/`) |
| `config.rs` | `~/.repartee/gui.toml` (minimal) |

## MVP scope (this build)

In:
- Connect to one server (`~/.repartee/gui.toml`, TLS), CAP/registration via
  irc-repartee, auto-join configured channels.
- Server / channel / query buffers; switch by clicking the sidebar.
- Render PRIVMSG, ACTION, NOTICE; JOIN/PART/QUIT/NICK/TOPIC events; numerics &
  MOTD to the server buffer.
- NAMES → nick list (prefix-sorted), per-nick colors.
- Send messages; slash commands `/join /part /msg /me /nick /quit` + raw
  passthrough. Local echo.
- Auto-scroll, mIRC code stripping.

Out (roadmap below).

## Roadmap (post-MVP, priority order)

1. **`rich_text` spans** → render mIRC colors/bold/italic inline (instead of
   stripping); clickable links.
2. **Reconnect** with backoff (currently single attempt).
3. **Connect/server wizard** (modal) + multi-server; replace the bare config
   file. Mirror repartee's wizard fields.
4. **SASL / TLS client-cert** UI.
5. **`repartee-core` extraction** → reuse full backend (storage/FTS, scripting,
   e2e, DCC).
6. **gg/Polish emotes** (`assets/emotes/`) + **image preview** — iced renders
   images natively.
7. **Themes** (load repartee `.theme` palettes), settings screen.
8. **Windows CI** build of `repartee-gui` (should pass by construction; prove
   it). Later: cfg-gate the TUI crate too if a unified binary is wanted.
9. Virtualized message log (halloy-style height cache) if perf needs it.

## Running (macOS)

```sh
cd repartee-app
cargo run -p repartee-gui
```

First launch writes `~/.repartee/gui.toml` (defaults: irc.libera.chat:6697 TLS,
nick `repartee<pid>`). Edit it to set your nick / `channels`, then relaunch.
Type `/join #channel` to start. Needs Rust + Xcode Command Line Tools.

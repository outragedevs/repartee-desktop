# repartee-gui

Native desktop GUI for [Repartee](https://repart.ee) — an **iced** frontend
(MVP). Halloy-style 3-pane layout (buffers · chat · nicks), rendered in our
bundled **FiraCode Nerd Font Mono**, with deterministic per-nick colors.

This crate is **Windows-clean by construction**: it depends on `irc-repartee`
plus a few of repartee's pure helpers, and does **not** link the terminal/Unix
parts of repartee (ratatui, crossterm, fork/daemon, PTY shell, Unix-socket
sessions). Design + roadmap: `../docs/superpowers/specs/2026-06-07-iced-gui-design.md`.

## Run (macOS / Linux / Windows)

```sh
cd repartee-app
cargo run -p repartee-gui
```

Requirements: Rust (stable) and, on macOS, Xcode Command Line Tools.

On first launch it writes `~/.repartee/gui.toml`:

```toml
server = "irc.libera.chat"
port = 6697
tls = true
nick = "reptee1234"
username = "repartee"
realname = "Repartee GUI (iced) — https://repart.ee"
channels = []        # e.g. ["#repartee"]
```

Edit it (set your nick / `channels`) and relaunch. Connection messages and MOTD
appear in the **(server)** buffer immediately. Then:

- `/join #channel` — join a channel (opens + focuses its buffer + nick list)
- type + Enter — send to the active channel/query
- `/msg nick text`, `/me action`, `/nick newnick`, `/part`, `/quit`
- any other `/VERB args` is sent raw
- click buffers in the left sidebar to switch

## Status

MVP: connect (1 server, TLS), buffers (server/channel/query), message rendering
(PRIVMSG/ACTION/NOTICE + JOIN/PART/QUIT/NICK/TOPIC events + numerics/MOTD), NAMES
→ nick list, sending + slash commands, auto-scroll, mIRC code stripping.

Not yet: inline mIRC colors (`rich_text`), reconnect, server wizard, SASL UI,
multi-server, e2e/scripting/DCC, emotes/image preview, themes. See the design
doc's roadmap.

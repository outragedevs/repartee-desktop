//! The iced application: state, messages, update loop, IRC routing.

use iced::widget::scrollable;
use iced::{Subscription, Task, Theme};

use crate::config::Config;
use crate::irc_client::{self, Event};
use crate::state::{Buffer, BufferKind, Line, LineKind};
use crate::{format, theme, ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Connecting,
    Connected,
    Disconnected,
}

/// Top-level application messages.
#[derive(Clone, Debug)]
pub enum Message {
    Irc(Event),
    InputChanged(String),
    Submit,
    SelectBuffer(usize),
}

pub struct App {
    pub cfg: Config,
    pub buffers: Vec<Buffer>,
    pub active: usize,
    pub input: String,
    pub sender: Option<irc::client::Sender>,
    pub nick: String,
    pub status: Status,
    pub log_id: scrollable::Id,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let cfg = Config::load_or_init();
        let nick = cfg.nick.clone();
        let app = Self {
            cfg,
            buffers: vec![Buffer::server()],
            active: 0,
            input: String::new(),
            sender: None,
            nick,
            status: Status::Connecting,
            log_id: scrollable::Id::new("chat-log"),
        };
        (app, Task::none())
    }

    pub fn title(&self) -> String {
        let net = &self.cfg.server;
        match self.status {
            Status::Connecting => format!("Repartee — connecting to {net}…"),
            Status::Connected => format!("Repartee — {net}"),
            Status::Disconnected => format!("Repartee — {net} (disconnected)"),
        }
    }

    pub fn theme(&self) -> Theme {
        theme::theme()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(irc_client::connect).map(Message::Irc)
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        ui::view(self)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // Any message that can append/select lines should pin the log to bottom.
        let scroll = matches!(
            message,
            Message::Irc(_) | Message::Submit | Message::SelectBuffer(_)
        );
        match message {
            Message::InputChanged(s) => self.input = s,
            Message::SelectBuffer(i) => {
                if i < self.buffers.len() {
                    self.active = i;
                }
            }
            Message::Submit => self.submit(),
            Message::Irc(ev) => self.handle_event(ev),
        }
        if scroll {
            scrollable::snap_to(self.log_id.clone(), scrollable::RelativeOffset::END)
        } else {
            Task::none()
        }
    }

    // --- IRC event handling ---

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Connected(sender, nick) => {
                self.sender = Some(sender);
                self.nick = nick;
                self.status = Status::Connected;
                self.server_event(format!("Connected to {} as {}", self.cfg.server, self.nick));
            }
            Event::Disconnected(reason) => {
                self.status = Status::Disconnected;
                self.server_event(format!("Disconnected: {reason}"));
            }
            Event::Message(msg) => self.handle_irc(*msg),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_irc(&mut self, msg: irc::proto::Message) {
        use irc::proto::Command;

        let src = format::extract_nick(msg.prefix.as_ref());
        let raw = msg.to_string();

        match &msg.command {
            Command::PRIVMSG(target, text) => {
                let (kind, body) = parse_action(text);
                let body = format::strip_irc_formatting(&body);
                let me = self.nick.clone();
                let buf = if format::is_channel(target) {
                    target.clone()
                } else {
                    // a query: bucket under the sender's nick
                    src.clone().unwrap_or_else(|| target.clone())
                };
                let bkind = if format::is_channel(&buf) {
                    BufferKind::Channel
                } else {
                    BufferKind::Query
                };
                let _ = me;
                let idx = self.ensure_buffer(&buf, bkind);
                self.buffers[idx].push(Line::now(kind, src, body));
            }
            Command::NOTICE(target, text) => {
                let body = format::strip_irc_formatting(text);
                let idx = if format::is_channel(target) {
                    self.ensure_buffer(target, BufferKind::Channel)
                } else {
                    0 // server buffer for server/user notices
                };
                self.buffers[idx].push(Line::now(LineKind::Notice, src, body));
            }
            Command::JOIN(chan, _, _) => {
                let idx = self.ensure_buffer(chan, BufferKind::Channel);
                if let Some(n) = &src {
                    self.buffers[idx].add_user(String::new(), n);
                    if n == &self.nick {
                        self.active = idx;
                        self.buffers[idx].push(Line::now(
                            LineKind::Event,
                            None,
                            format!("Joined {chan}"),
                        ));
                    } else {
                        self.buffers[idx].push(Line::now(
                            LineKind::Event,
                            None,
                            format!("→ {n} joined"),
                        ));
                    }
                }
            }
            Command::PART(chan, reason) => {
                let idx = self.ensure_buffer(chan, BufferKind::Channel);
                if let Some(n) = &src {
                    self.buffers[idx].remove_user(n);
                    let r = reason.clone().unwrap_or_default();
                    self.buffers[idx].push(Line::now(
                        LineKind::Event,
                        None,
                        format!(
                            "← {n} left {}",
                            if r.is_empty() {
                                String::new()
                            } else {
                                format!("({r})")
                            }
                        ),
                    ));
                }
            }
            Command::QUIT(reason) => {
                if let Some(n) = &src {
                    let r = reason.clone().unwrap_or_default();
                    let msg = format!(
                        "⤫ {n} quit{}",
                        if r.is_empty() {
                            String::new()
                        } else {
                            format!(" ({r})")
                        }
                    );
                    for b in &mut self.buffers {
                        if b.has_user(n) {
                            b.remove_user(n);
                            b.push(Line::now(LineKind::Event, None, msg.clone()));
                        }
                    }
                }
            }
            Command::NICK(newnick) => {
                if let Some(old) = &src {
                    if old == &self.nick {
                        self.nick = newnick.clone();
                    }
                    let msg = format!("{old} is now {newnick}");
                    for b in &mut self.buffers {
                        if b.has_user(old) {
                            b.rename_user(old, newnick);
                            b.push(Line::now(LineKind::Event, None, msg.clone()));
                        }
                    }
                }
            }
            Command::TOPIC(chan, topic) => {
                let idx = self.ensure_buffer(chan, BufferKind::Channel);
                if let Some(t) = topic {
                    self.buffers[idx].topic = format::strip_irc_formatting(t);
                }
            }
            Command::Response(resp, args) => self.handle_response(*resp, args, &raw),
            _ => {
                // Anything else (MODE, PING/PONG handled by the lib, etc.) →
                // show the raw line in the server buffer so nothing is lost.
                self.buffers[0].push(Line::now(LineKind::Event, None, raw.trim_end().to_string()));
            }
        }
    }

    fn handle_response(&mut self, resp: irc::proto::Response, args: &[String], raw: &str) {
        use irc::proto::Response;

        match resp {
            Response::RPL_WELCOME => {
                if let Some(n) = args.first() {
                    self.nick = n.clone();
                }
                self.server_event(args.last().cloned().unwrap_or_default());
            }
            Response::RPL_TOPIC => {
                // args = [me, #chan, topic]
                if let (Some(chan), Some(topic)) = (args.get(1), args.get(2)) {
                    let idx = self.ensure_buffer(chan, BufferKind::Channel);
                    self.buffers[idx].topic = format::strip_irc_formatting(topic);
                }
            }
            Response::RPL_NAMREPLY => {
                // args = [me, "=", #chan, "n1 n2 ..."]
                if let (Some(chan), Some(names)) = (args.get(2), args.get(3)) {
                    let idx = self.ensure_buffer(chan, BufferKind::Channel);
                    for entry in names.split_whitespace() {
                        let (prefix, nick) = format::split_nick_prefix(entry);
                        self.buffers[idx].add_user(prefix, nick);
                    }
                }
            }
            Response::RPL_ENDOFNAMES => {}
            _ => {
                // Generic numeric → server buffer. Drop the leading target
                // (our nick) if present for readability.
                let text = if args.len() > 1 {
                    args[1..].join(" ")
                } else if args.is_empty() {
                    raw.trim_end().to_string()
                } else {
                    args.join(" ")
                };
                self.server_event(text);
            }
        }
    }

    // --- sending ---

    fn submit(&mut self) {
        let text = self.input.trim().to_string();
        self.input.clear();
        if text.is_empty() {
            return;
        }
        let Some(sender) = self.sender.clone() else {
            self.server_event("Not connected yet.".to_string());
            return;
        };

        if let Some(rest) = text.strip_prefix('/') {
            self.run_command(&sender, rest);
        } else {
            let target = self.buffers[self.active].name.clone();
            if self.buffers[self.active].kind == BufferKind::Server {
                self.server_event("No channel/query active — use /join #channel".to_string());
                return;
            }
            if sender.send_privmsg(&target, &text).is_ok() {
                let me = self.nick.clone();
                self.buffers[self.active].push(Line::now(LineKind::Message, Some(me), text));
            }
        }
    }

    fn run_command(&mut self, sender: &irc::client::Sender, rest: &str) {
        use irc::proto::Command;

        let mut parts = rest.splitn(2, ' ');
        let verb = parts.next().unwrap_or_default().to_lowercase();
        let arg = parts.next().unwrap_or_default().trim();

        match verb.as_str() {
            "join" | "j" => {
                if !arg.is_empty() {
                    let _ = sender.send_join(arg);
                }
            }
            "part" => {
                let chan = if arg.is_empty() {
                    self.buffers[self.active].name.clone()
                } else {
                    arg.to_string()
                };
                let _ = sender.send_part(chan);
            }
            "msg" | "query" => {
                let mut it = arg.splitn(2, ' ');
                if let (Some(t), Some(m)) = (it.next(), it.next())
                    && sender.send_privmsg(t, m).is_ok()
                {
                    let idx = self.ensure_buffer(t, BufferKind::Query);
                    let me = self.nick.clone();
                    self.buffers[idx].push(Line::now(LineKind::Message, Some(me), m.to_string()));
                }
            }
            "me" => {
                let target = self.buffers[self.active].name.clone();
                if self.buffers[self.active].kind != BufferKind::Server && !arg.is_empty() {
                    let ctcp = format!("\u{1}ACTION {arg}\u{1}");
                    if sender.send_privmsg(&target, ctcp).is_ok() {
                        let me = self.nick.clone();
                        self.buffers[self.active].push(Line::now(
                            LineKind::Action,
                            Some(me),
                            arg.to_string(),
                        ));
                    }
                }
            }
            "nick" => {
                if !arg.is_empty() {
                    let _ = sender.send(Command::NICK(arg.to_string()));
                }
            }
            "quit" => {
                let _ = sender.send(Command::QUIT(if arg.is_empty() {
                    None
                } else {
                    Some(arg.to_string())
                }));
            }
            _ => {
                // Raw passthrough: VERB rest...
                let args: Vec<String> = arg.split_whitespace().map(String::from).collect();
                let _ = sender.send(Command::Raw(verb.to_uppercase(), args));
            }
        }
    }

    // --- helpers ---

    fn server_event(&mut self, text: String) {
        self.buffers[0].push(Line::now(LineKind::Event, None, text));
    }

    /// Find a buffer by name (case-insensitive), creating it if missing.
    fn ensure_buffer(&mut self, name: &str, kind: BufferKind) -> usize {
        if let Some(i) = self
            .buffers
            .iter()
            .position(|b| b.name.eq_ignore_ascii_case(name))
        {
            return i;
        }
        self.buffers.push(Buffer::new(name, kind));
        self.buffers.len() - 1
    }
}

/// Detect a CTCP ACTION (`\x01ACTION ...\x01`); returns the line kind + body.
fn parse_action(text: &str) -> (LineKind, String) {
    if let Some(inner) = text.strip_prefix('\u{1}') {
        let inner = inner.strip_suffix('\u{1}').unwrap_or(inner);
        if let Some(action) = inner.strip_prefix("ACTION ") {
            return (LineKind::Action, action.to_string());
        }
    }
    (LineKind::Message, text.to_string())
}

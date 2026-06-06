//! UI-agnostic GUI state: buffers, lines, users. A deliberately small subset of
//! repartee's `src/state/` tailored to the MVP; the design doc covers folding in
//! the full state layer later.

use chrono::{DateTime, Local};

use crate::format;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferKind {
    Server,
    Channel,
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Message,
    Action,
    Notice,
    Event,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub time: DateTime<Local>,
    pub kind: LineKind,
    pub nick: Option<String>,
    pub text: String,
}

impl Line {
    pub fn now(kind: LineKind, nick: Option<String>, text: String) -> Self {
        Self {
            time: Local::now(),
            kind,
            nick,
            text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub nick: String,
    pub prefix: String,
}

#[derive(Debug, Clone)]
pub struct Buffer {
    pub name: String,
    pub kind: BufferKind,
    pub topic: String,
    pub lines: Vec<Line>,
    pub users: Vec<User>,
}

impl Buffer {
    pub fn new(name: impl Into<String>, kind: BufferKind) -> Self {
        Self {
            name: name.into(),
            kind,
            topic: String::new(),
            lines: Vec::new(),
            users: Vec::new(),
        }
    }

    pub fn server() -> Self {
        Self::new("(server)", BufferKind::Server)
    }

    pub fn push(&mut self, line: Line) {
        // Keep memory bounded for the MVP.
        const CAP: usize = 5_000;
        if self.lines.len() >= CAP {
            self.lines.drain(0..self.lines.len() - CAP + 1);
        }
        self.lines.push(line);
    }

    pub fn add_user(&mut self, prefix: String, nick: &str) {
        if let Some(u) = self.users.iter_mut().find(|u| u.nick == nick) {
            u.prefix = prefix;
        } else {
            self.users.push(User {
                nick: nick.to_string(),
                prefix,
            });
        }
        self.sort_users();
    }

    pub fn remove_user(&mut self, nick: &str) {
        self.users.retain(|u| u.nick != nick);
    }

    pub fn rename_user(&mut self, old: &str, new: &str) {
        if let Some(u) = self.users.iter_mut().find(|u| u.nick == old) {
            u.nick = new.to_string();
            self.sort_users();
        }
    }

    pub fn has_user(&self, nick: &str) -> bool {
        self.users.iter().any(|u| u.nick == nick)
    }

    fn sort_users(&mut self) {
        self.users.sort_by(|a, b| {
            format::prefix_rank(&a.prefix)
                .cmp(&format::prefix_rank(&b.prefix))
                .then_with(|| a.nick.to_lowercase().cmp(&b.nick.to_lowercase()))
        });
    }
}

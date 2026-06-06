//! IRC text helpers: deterministic per-nick colors and mIRC formatting strip.
//!
//! Ported from repartee's `src/nick_color.rs` and `src/irc/formatting.rs`
//! (same project, MIT) — the only change is returning `iced::Color` instead of
//! `ratatui::style::Color`, so this stays free of any terminal dependency.

use iced::Color;

/// Strip mIRC formatting codes (`\x02` bold, `\x03` color, `\x04` hex color,
/// `\x0F` reset, `\x16` reverse, `\x1D` italic, `\x1E` strike, `\x1F` underline)
/// from `text`, returning plain text. UTF-8 safe.
#[must_use]
pub fn strip_irc_formatting(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        match bytes[i] {
            b'\x02' | b'\x1D' | b'\x1F' | b'\x1E' | b'\x16' | b'\x0F' => i += 1,
            b'\x03' => {
                i += 1;
                let mut digits = 0;
                while i < len && bytes[i].is_ascii_digit() && digits < 2 {
                    i += 1;
                    digits += 1;
                }
                if i < len && bytes[i] == b',' && i + 1 < len && bytes[i + 1].is_ascii_digit() {
                    i += 1;
                    let mut bg = 0;
                    while i < len && bytes[i].is_ascii_digit() && bg < 2 {
                        i += 1;
                        bg += 1;
                    }
                }
            }
            b'\x04' => {
                i += 1;
                let mut hex = 0;
                while i < len && bytes[i].is_ascii_hexdigit() && hex < 6 {
                    i += 1;
                    hex += 1;
                }
                if i < len && bytes[i] == b',' && i + 1 < len && bytes[i + 1].is_ascii_hexdigit() {
                    i += 1;
                    let mut bg = 0;
                    while i < len && bytes[i].is_ascii_hexdigit() && bg < 6 {
                        i += 1;
                        bg += 1;
                    }
                }
            }
            _ => {
                let start = i;
                i += 1;
                while i < len && (bytes[i] & 0xC0) == 0x80 {
                    i += 1;
                }
                out.push_str(&text[start..i]);
            }
        }
    }
    out
}

/// djb2 string hash, case-insensitive — fast, good distribution for short nicks.
const fn djb2_hash(nick: &str) -> usize {
    let bytes = nick.as_bytes();
    let mut hash: u32 = 5381;
    let mut idx = 0;
    while idx < bytes.len() {
        let lower = bytes[idx].to_ascii_lowercase();
        hash = hash.wrapping_mul(33).wrapping_add(lower as u32);
        idx += 1;
    }
    hash as usize
}

/// Convert HSL (`hue` 0..360, `saturation`/`lightness` 0..1) to RGB bytes.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0f32.mul_add(lightness, -1.0)).abs()) * saturation;
    let h_prime = hue / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = lightness - c / 2.0;
    let red = (r1 + m).mul_add(255.0, 0.5) as u8;
    let green = (g1 + m).mul_add(255.0, 0.5) as u8;
    let blue = (b1 + m).mul_add(255.0, 0.5) as u8;
    (red, green, blue)
}

/// Deterministic per-nick color (case-insensitive), as an `iced::Color`.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn nick_color(nick: &str, saturation: f32, lightness: f32) -> Color {
    let hash = djb2_hash(nick);
    let hue = (hash % 360) as f32;
    let (r, g, b) = hsl_to_rgb(hue, saturation, lightness);
    Color::from_rgb8(r, g, b)
}

/// Whether `target` is a channel (`#`, `&`, `+`, `!`).
#[must_use]
pub fn is_channel(target: &str) -> bool {
    target.starts_with(['#', '&', '+', '!'])
}

/// Split leading status prefixes (`~&@%+`) from a NAMES entry, returning
/// `(prefix, nick)`.
#[must_use]
pub fn split_nick_prefix(entry: &str) -> (String, &str) {
    const PREFIXES: &str = "~&@%+";
    let mut prefix = String::new();
    let mut start = 0;
    for (i, c) in entry.char_indices() {
        if PREFIXES.contains(c) {
            prefix.push(c);
            start = i + c.len_utf8();
        } else {
            break;
        }
    }
    (prefix, &entry[start..])
}

/// Extract the nick (or server name) from an IRC prefix.
#[must_use]
pub fn extract_nick(prefix: Option<&irc::proto::Prefix>) -> Option<String> {
    use irc::proto::Prefix;
    prefix.map(|p| match p {
        Prefix::Nickname(nick, _, _) => nick.clone(),
        Prefix::ServerName(name) => name.clone(),
    })
}

/// Rank of the highest status prefix for sorting (lower = higher rank).
#[must_use]
pub fn prefix_rank(prefix: &str) -> u8 {
    for (rank, c) in "~&@%+".chars().enumerate() {
        if prefix.contains(c) {
            #[allow(clippy::cast_possible_truncation)]
            return rank as u8;
        }
    }
    u8::MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_codes() {
        assert_eq!(strip_irc_formatting("\x02\x034,2\x1Fhi\x0F there"), "hi there");
    }

    #[test]
    fn nick_color_deterministic_and_case_insensitive() {
        assert_eq!(nick_color("Ferris", 0.65, 0.65), nick_color("ferris", 0.65, 0.65));
    }

    #[test]
    fn channel_detection() {
        assert!(is_channel("#rust"));
        assert!(!is_channel("alice"));
    }

    #[test]
    fn prefix_split() {
        assert_eq!(split_nick_prefix("@+bob"), ("@+".to_string(), "bob"));
        assert_eq!(split_nick_prefix("alice"), (String::new(), "alice"));
    }

    #[test]
    fn rank_order() {
        assert!(prefix_rank("@") < prefix_rank("+"));
        assert!(prefix_rank("+") < prefix_rank(""));
    }
}

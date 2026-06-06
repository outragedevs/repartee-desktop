//! Visual theme: bundled FiraCode font + a warm-dark palette in the spirit of
//! halloy's "ferra", but rendered in our monospace FiraCode Nerd Font.

use iced::theme::Palette;
use iced::{Color, Font, Theme, font};

/// Internal family name of the bundled font (reported by FiraCode Nerd Font Mono).
pub const FONT_NAME: &str = "FiraCode Nerd Font Mono";

/// Bundled font bytes (restored under `assets/fonts/`, shared with the desktop build).
pub const FIRA_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/FiraCodeNerdFontMono-Regular.ttf");
pub const FIRA_BOLD: &[u8] = include_bytes!("../../assets/fonts/FiraCodeNerdFontMono-Bold.ttf");

/// The default (regular) font.
#[must_use]
pub fn font() -> Font {
    Font::with_name(FONT_NAME)
}

/// The bold variant of the bundled font.
#[must_use]
pub fn bold() -> Font {
    Font {
        weight: font::Weight::Bold,
        ..Font::with_name(FONT_NAME)
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

// --- palette (warm dark) ---
pub const BG: Color = rgb(0x1c, 0x18, 0x1b); // main background
pub const BG_PANEL: Color = rgb(0x24, 0x1f, 0x23); // sidebars / panels
pub const BG_ACTIVE: Color = rgb(0x34, 0x2c, 0x32); // selected buffer
pub const TEXT: Color = rgb(0xe0, 0xd9, 0xd4); // primary text
pub const DIM: Color = rgb(0x8a, 0x7f, 0x82); // timestamps / muted
pub const ACCENT: Color = rgb(0xd9, 0x7c, 0x6a); // ferra-ish coral accent
pub const GREEN: Color = rgb(0x8f, 0xb5, 0x8a);
pub const RED: Color = rgb(0xe0, 0x6c, 0x75);
pub const BORDER: Color = rgb(0x39, 0x31, 0x37);

/// Per-nick color tuning (matches repartee defaults closely).
pub const NICK_SAT: f32 = 0.55;
pub const NICK_LIT: f32 = 0.70;

/// Deterministic color for a nick, using our saturation/lightness defaults.
#[must_use]
pub fn nick_for(nick: &str) -> Color {
    crate::format::nick_color(nick, NICK_SAT, NICK_LIT)
}

/// The app theme — a custom palette so iced's stock widgets match our colors.
#[must_use]
pub fn theme() -> Theme {
    Theme::custom(
        "Repartee".to_string(),
        Palette {
            background: BG,
            text: TEXT,
            primary: ACCENT,
            success: GREEN,
            danger: RED,
        },
    )
}

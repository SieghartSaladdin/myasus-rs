use rune::tok::Family;
use rune::{Anim, Color, El, Spring, TextStyle, Transition, col, el, row, text};
use rune_kit::{Kit, PaletteSeed};

/// Converts an 8-bit sRGB color into Rune's linear sRGB `Color`.
pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: rune::theme::srgb_decode(r as f32 / 255.0),
        g: rune::theme::srgb_decode(g as f32 / 255.0),
        b: rune::theme::srgb_decode(b as f32 / 255.0),
        a: 1.0,
    }
}

// True OLED-friendly obsidian palette (deep blacks save OLED power & reduce burn-in)
pub fn asus_blue() -> Color {
    rgb(22, 132, 255)
}

/// `asus_blue` text on its own 15% tint is 4.1:1; this lighter blue is 6:1 there.
pub fn asus_blue_soft() -> Color {
    rgb(92, 168, 255)
}

pub fn oled_teal() -> Color {
    rgb(32, 206, 166)
}

pub fn bg_canvas() -> Color {
    rgb(11, 13, 18)
}

pub fn bg_sidebar() -> Color {
    rgb(15, 18, 24)
}

pub fn bg_card() -> Color {
    rgb(20, 24, 32)
}

pub fn bg_elevated() -> Color {
    rgb(28, 33, 44)
}

pub fn border_subtle() -> Color {
    rgb(42, 48, 62)
}

pub fn text_primary() -> Color {
    rgb(240, 244, 250)
}

pub fn text_muted() -> Color {
    rgb(146, 156, 174)
}

pub fn status_ok() -> Color {
    rgb(46, 212, 128)
}

pub fn status_warn() -> Color {
    rgb(245, 182, 66)
}

pub fn status_danger() -> Color {
    rgb(238, 96, 96)
}

pub fn oled_notice_red() -> Color {
    rgb(232, 114, 104)
}

/// Builds the OLED-native dark `rune_kit::Kit`.
pub fn myasus_kit() -> Kit {
    let mut k = Kit::from_palette(PaletteSeed {
        bg: bg_canvas(),
        fg: text_primary(),
        accent: asus_blue(),
    });
    k.c.bg_accent = asus_blue();
    k.c.border_focus = asus_blue();
    k.radius.sm = 6.0;
    k.radius.md = 8.0;
    k.radius.lg = 12.0;
    k
}

pub fn font_style(size: f32, weight: f32) -> TextStyle {
    TextStyle {
        line_height: size * 1.35,
        ..TextStyle::new(size, weight)
    }
}

pub fn font_mono(size: f32, weight: f32) -> TextStyle {
    TextStyle {
        family: Family::Mono,
        line_height: size * 1.35,
        ..TextStyle::new(size, weight)
    }
}

pub fn page_title(title: impl Into<String>, subtitle: impl Into<String>) -> El {
    col()
        .gap(3.0)
        .shrink()
        .min_w(0.0)
        .child(
            text(title.into())
                .font(font_style(22.0, 700.0))
                .color(text_primary())
                .wrap()
                .w_full(),
        )
        .child(
            text(subtitle.into())
                .font(font_style(12.5, 420.0))
                .color(text_muted())
                .wrap()
                .w_full(),
        )
}

pub fn surface_card(children: Vec<El>) -> El {
    col()
        .w_full()
        .gap(10.0)
        .padding(15.0)
        .radius(10.0)
        .fill(bg_card())
        .border(1.0, border_subtle())
        .children(children)
}

pub fn card_heading(title: impl Into<String>, subtitle: Option<&str>) -> El {
    let mut c = col().gap(2.0).shrink().min_w(0.0).child(
        text(title.into())
            .font(font_style(15.0, 680.0))
            .color(text_primary())
            .wrap()
            .w_full(),
    );
    if let Some(sub) = subtitle {
        c = c.child(
            text(sub.to_string())
                .font(font_style(12.0, 400.0))
                .color(text_muted())
                .wrap()
                .w_full(),
        );
    }
    c
}

pub fn status_pill(label: impl Into<String>, color: Color) -> El {
    row()
        .center_y()
        .gap(6.0)
        .px(9.0)
        .py(3.5)
        .radius(999.0)
        .fill(color.alpha(0.15))
        .border(1.0, color.alpha(0.45))
        .child(el().size(6.0, 6.0).radius(3.0).fill(color))
        .child(text(label.into()).font(font_style(11.0, 620.0)).color(color))
}

/// A value shown in a capsule (a limit, a governor, an offset). No dot: a dot says "this is a
/// state", and these only carry a number or a name.
pub fn value_pill(label: impl Into<String>, color: Color) -> El {
    row()
        .center_y()
        .px(9.0)
        .py(3.5)
        .radius(999.0)
        .fill(color.alpha(0.15))
        .border(1.0, color.alpha(0.45))
        .child(text(label.into()).font(font_style(11.0, 620.0)).color(color))
}

pub fn meter_bar(frac: f32, color: Color) -> El {
    let clamped = frac.clamp(0.0, 1.0);
    el()
        .w_full()
        .h(7.0)
        .radius(4.0)
        .fill(bg_elevated())
        .child(el().w_frac(clamped).h_full().radius(4.0).fill(color))
}

pub fn kv_row(label: impl Into<String>, value: impl Into<String>) -> El {
    row()
        .w_full()
        .center_y()
        .justify_between()
        .py(6.0)
        .border_sides([0.0, 0.0, 1.0, 0.0], border_subtle().alpha(0.5))
        .child(
            text(label.into())
                .font(font_style(12.5, 500.0))
                .color(text_muted()),
        )
        .child(
            text(value.into())
                .font(font_style(12.5, 580.0))
                .color(text_primary())
                .wrap()
                .shrink()
                .min_w(0.0),
        )
}

pub fn divider() -> El {
    el().w_full().h(1.0).fill(border_subtle().alpha(0.75))
}

/// Wraps a page or tab body so switching views swaps it as one keyed unit: the old view leaves
/// at once and the new one rises 8pt while fading in. Without a distinct key per view, Rune
/// matches the two views' elements by position and morphs unrelated cards into each other.
pub fn swap_view(key: impl Into<String>, body: El) -> El {
    el()
        .key(key.into())
        .w_full()
        .transition(Transition::offset(0.0, 8.0).combined(Transition::opacity()))
        .transition_motion(Spring::new(0.28, 0.0), Anim::INSTANT)
        .child(body)
}

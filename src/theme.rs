use egui::{Color32, Rounding, Stroke, Visuals};

// A warm, dim ramp: this app lives next to patch panels and indicator
// lamps, not SaaS dashboards, so the neutral tilts warm instead of blue-grey.
pub const BG: Color32 = Color32::from_rgb(0x16, 0x15, 0x13);
pub const CARD: Color32 = Color32::from_rgb(0x1c, 0x1a, 0x18);
pub const RAISED: Color32 = Color32::from_rgb(0x26, 0x24, 0x21);
pub const HAIRLINE: Color32 = Color32::from_rgb(0x2b, 0x28, 0x24);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xef, 0xe9, 0xde);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xaa, 0xa3, 0x98);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8a, 0x84, 0x7a);

/// The one accent in the whole app: a warm indicator-lamp amber. It marks
/// exactly one thing, which account is currently signed in, and nothing else.
pub const ACCENT: Color32 = Color32::from_rgb(0xe0, 0xa9, 0x4c);
pub const POSITIVE: Color32 = Color32::from_rgb(0x4c, 0xc3, 0x8a);
pub const NEGATIVE: Color32 = Color32::from_rgb(0xe5, 0x48, 0x4d);

pub const RADIUS_ROW: f32 = 8.0;
pub const RADIUS_CARD: f32 = 12.0;

pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 16.0;

pub fn apply(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.window_fill = BG;
    visuals.panel_fill = BG;
    visuals.faint_bg_color = CARD;
    visuals.extreme_bg_color = RAISED;
    visuals.warn_fg_color = ACCENT;
    visuals.error_fg_color = NEGATIVE;
    visuals.hyperlink_color = ACCENT;

    visuals.widgets.noninteractive.bg_fill = CARD;
    visuals.widgets.noninteractive.weak_bg_fill = CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, HAIRLINE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.noninteractive.rounding = Rounding::same(RADIUS_CARD);

    visuals.widgets.inactive.bg_fill = RAISED;
    visuals.widgets.inactive.weak_bg_fill = RAISED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, HAIRLINE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(RADIUS_ROW);

    visuals.widgets.hovered.bg_fill = RAISED;
    visuals.widgets.hovered.weak_bg_fill = RAISED;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, TEXT_MUTED);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(RADIUS_ROW);

    visuals.widgets.active.bg_fill = RAISED;
    visuals.widgets.active.weak_bg_fill = RAISED;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.active.rounding = Rounding::same(RADIUS_ROW);

    visuals.selection.bg_fill = RAISED;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    visuals.window_rounding = Rounding::same(RADIUS_CARD);
    visuals.window_stroke = Stroke::new(1.0, HAIRLINE);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(SPACE_SM, SPACE_SM);
    style.spacing.button_padding = egui::vec2(SPACE_MD, SPACE_XS);
    style.spacing.window_margin = SPACE_LG.into();
    ctx.set_style(style);
}

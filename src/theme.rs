use egui::{Color32, Margin, Rounding, Stroke, Style, Visuals};

pub const COLOR_BG_MAIN: Color32 = Color32::from_rgb(15, 17, 21);
pub const COLOR_BG_SIDEBAR: Color32 = Color32::from_rgb(19, 21, 27);
pub const COLOR_BG_CARD: Color32 = Color32::from_rgb(24, 26, 33);
pub const COLOR_BG_SURFACE: Color32 = Color32::from_rgb(32, 35, 45);
pub const COLOR_BG_INPUT: Color32 = Color32::from_rgb(18, 20, 26);
pub const COLOR_BORDER: Color32 = Color32::from_rgb(38, 42, 54);

pub const COLOR_PRIMARY: Color32 = Color32::from_rgb(16, 185, 129);
pub const COLOR_PRIMARY_HOVER: Color32 = Color32::from_rgb(5, 150, 105);
pub const COLOR_PURPLE: Color32 = Color32::from_rgb(168, 85, 247);
pub const COLOR_RED: Color32 = Color32::from_rgb(239, 68, 68);
pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(243, 244, 246);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(140, 147, 164);

pub fn apply_catrinth_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.override_text_color = Some(COLOR_TEXT_PRIMARY);
    visuals.panel_fill = COLOR_BG_MAIN;
    visuals.window_fill = COLOR_BG_CARD;
    visuals.window_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.window_rounding = Rounding::same(12.0);

    visuals.widgets.noninteractive.bg_fill = COLOR_BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_PRIMARY);

    visuals.widgets.inactive.bg_fill = COLOR_BG_SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.inactive.rounding = Rounding::same(8.0);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_PRIMARY);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 50, 72);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, COLOR_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(8.0);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    visuals.widgets.active.bg_fill = COLOR_PRIMARY_HOVER;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, COLOR_PRIMARY);
    visuals.widgets.active.rounding = Rounding::same(8.0);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    visuals.widgets.open.bg_fill = COLOR_BG_SURFACE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, COLOR_PRIMARY);
    visuals.widgets.open.rounding = Rounding::same(8.0);

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(16, 185, 129, 60);
    visuals.selection.stroke = Stroke::new(1.0_f32, COLOR_PRIMARY);

    let mut style = Style::clone(&ctx.style());
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = Margin::same(16.0);

    ctx.set_style(style);
}

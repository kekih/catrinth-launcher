use egui::{Color32, CornerRadius, Margin, Rounding, Stroke, Style, Visuals};

pub const COLOR_BG_MAIN: Color32 = Color32::from_rgb(11, 14, 20);
pub const COLOR_BG_CARD: Color32 = Color32::from_rgb(19, 23, 34);
pub const COLOR_BG_SURFACE: Color32 = Color32::from_rgb(32, 39, 56);
pub const COLOR_BG_INPUT: Color32 = Color32::from_rgb(15, 18, 26);
pub const COLOR_BORDER: Color32 = Color32::from_rgb(36, 44, 61);

pub const COLOR_PRIMARY: Color32 = Color32::from_rgb(16, 185, 129);
pub const COLOR_PRIMARY_HOVER: Color32 = Color32::from_rgb(5, 150, 105);
pub const COLOR_PURPLE: Color32 = Color32::from_rgb(139, 92, 246);
pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(248, 250, 252);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);

pub fn apply_catrinth_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.override_text_color = Some(COLOR_TEXT_PRIMARY);
    visuals.panel_fill = COLOR_BG_MAIN;
    visuals.window_fill = COLOR_BG_CARD;
    visuals.window_stroke = Stroke::new(1.0, COLOR_BORDER);
    visuals.window_corner_radius = CornerRadius::same(12);

    visuals.widgets.noninteractive.bg_fill = COLOR_BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, COLOR_BORDER);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(8);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, COLOR_TEXT_PRIMARY);

    visuals.widgets.inactive.bg_fill = COLOR_BG_SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, COLOR_BORDER);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, COLOR_TEXT_PRIMARY);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 50, 72);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, COLOR_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.widgets.active.bg_fill = COLOR_PRIMARY_HOVER;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, COLOR_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.widgets.open.bg_fill = COLOR_BG_SURFACE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, COLOR_PRIMARY);
    visuals.widgets.open.corner_radius = CornerRadius::same(8);

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(16, 185, 129, 60);
    visuals.selection.stroke = Stroke::new(1.0, COLOR_PRIMARY);

    let mut style = Style::clone(&ctx.style());
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = Margin::same(16);

    ctx.set_style(style);
}

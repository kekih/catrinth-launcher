use egui::{Color32, Margin, Rounding, Stroke, Style, Visuals};

// ============================================================================
// MODRINTH APP (THESEUS) MODERN DARK THEME PALETTE - HIGH CONTRAST & READABLE
// ============================================================================

// Backgrounds & Surfaces
pub const COLOR_BG_MAIN: Color32 = Color32::from_rgb(18, 19, 24);       // #121318 (Deep Slate)
pub const COLOR_BG_SIDEBAR: Color32 = Color32::from_rgb(14, 15, 19);    // #0e0f13 (Sidebar Dark)
pub const COLOR_BG_CARD: Color32 = Color32::from_rgb(25, 27, 34);       // #191b22 (Card base)
pub const COLOR_BG_SURFACE: Color32 = Color32::from_rgb(34, 37, 47);    // #22252f (Elevated surface)
pub const COLOR_BG_HOVER: Color32 = Color32::from_rgb(42, 46, 58);      // #2a2e3a (Hover fill)
pub const COLOR_BG_INPUT: Color32 = Color32::from_rgb(20, 22, 28);      // #14161c (Input field fill)
pub const COLOR_BORDER: Color32 = Color32::from_rgb(46, 50, 64);        // #2e3240 (Crisp border)
pub const COLOR_BORDER_LIGHT: Color32 = Color32::from_rgb(62, 68, 86);  // #3e4456 (Prominent border)

// Modrinth Brand Green Accents
pub const COLOR_PRIMARY: Color32 = Color32::from_rgb(27, 217, 106);     // #1bd96a (Modrinth Emerald)
pub const COLOR_PRIMARY_HOVER: Color32 = Color32::from_rgb(40, 245, 125); // #28f57d
pub const COLOR_PRIMARY_DARK: Color32 = Color32::from_rgb(18, 165, 78);   // #12a54e
pub const COLOR_PRIMARY_BG: Color32 = Color32::from_rgba_premultiplied(27, 217, 106, 35);

// Mod Loaders & Status Badges
pub const COLOR_PURPLE: Color32 = Color32::from_rgb(175, 95, 255);      // #af5fff (Fabric Violet)
pub const COLOR_PURPLE_BG: Color32 = Color32::from_rgba_premultiplied(175, 95, 255, 35);
pub const COLOR_ORANGE: Color32 = Color32::from_rgb(255, 135, 30);       // #ff871e (Forge Amber)
pub const COLOR_ORANGE_BG: Color32 = Color32::from_rgba_premultiplied(255, 135, 30, 35);
pub const COLOR_BLUE: Color32 = Color32::from_rgb(65, 145, 255);        // #4191ff (NeoForge / Blue)
pub const COLOR_BLUE_BG: Color32 = Color32::from_rgba_premultiplied(65, 145, 255, 35);
pub const COLOR_RED: Color32 = Color32::from_rgb(245, 75, 75);          // #f54b4b (Danger / Stop)
pub const COLOR_RED_BG: Color32 = Color32::from_rgba_premultiplied(245, 75, 75, 35);

// High-Contrast Typography (Crisp & Legible)
pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 255, 255); // Pure white headers
pub const COLOR_TEXT_BODY: Color32 = Color32::from_rgb(235, 238, 245);    // Crisp white-gray labels
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(168, 176, 194);   // Highly readable secondary text
pub const COLOR_TEXT_SUBTLE: Color32 = Color32::from_rgb(132, 140, 158);  // Hints and descriptions

pub fn apply_catrinth_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.override_text_color = Some(COLOR_TEXT_BODY);
    visuals.panel_fill = COLOR_BG_MAIN;
    visuals.window_fill = COLOR_BG_CARD;
    visuals.window_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.window_rounding = Rounding::same(12.0);

    visuals.widgets.noninteractive.bg_fill = COLOR_BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_BODY);

    visuals.widgets.inactive.bg_fill = COLOR_BG_SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.inactive.rounding = Rounding::same(8.0);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_BODY);

    visuals.widgets.hovered.bg_fill = COLOR_BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, COLOR_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(8.0);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_PRIMARY);

    visuals.widgets.active.bg_fill = COLOR_PRIMARY_DARK;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, COLOR_PRIMARY_HOVER);
    visuals.widgets.active.rounding = Rounding::same(8.0);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(8, 20, 12));

    visuals.widgets.open.bg_fill = COLOR_BG_SURFACE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, COLOR_PRIMARY);
    visuals.widgets.open.rounding = Rounding::same(8.0);

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(27, 217, 106, 60);
    visuals.selection.stroke = Stroke::new(1.0_f32, COLOR_PRIMARY);

    let mut style = Style::clone(&ctx.style());
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = Margin::same(16.0);

    ctx.set_style(style);
}

use egui::{Color32, Margin, Rounding, Stroke, Style, Visuals};

// ============================================================================
// MODRINTH APP (THESEUS) MODERN DARK THEME PALETTE
// ============================================================================

// Backgrounds & Surfaces
pub const COLOR_BG_MAIN: Color32 = Color32::from_rgb(17, 18, 22);       // #111216 (Deep Slate)
pub const COLOR_BG_SIDEBAR: Color32 = Color32::from_rgb(13, 14, 17);    // #0d0e11 (Dark Sidebar)
pub const COLOR_BG_CARD: Color32 = Color32::from_rgb(23, 25, 32);       // #171920 (Modrinth Card)
pub const COLOR_BG_SURFACE: Color32 = Color32::from_rgb(30, 33, 43);    // #1e212b (Elevated surface)
pub const COLOR_BG_HOVER: Color32 = Color32::from_rgb(36, 40, 52);      // #242834 (Hover fill)
pub const COLOR_BG_INPUT: Color32 = Color32::from_rgb(19, 20, 26);      // #13141a (Input bg)
pub const COLOR_BORDER: Color32 = Color32::from_rgb(38, 41, 54);        // #262936 (Crisp border)
pub const COLOR_BORDER_LIGHT: Color32 = Color32::from_rgb(52, 57, 75);  // #34394b

// Modrinth Brand Accents
pub const COLOR_PRIMARY: Color32 = Color32::from_rgb(27, 217, 106);     // #1bd96a (Modrinth Emerald)
pub const COLOR_PRIMARY_HOVER: Color32 = Color32::from_rgb(37, 240, 120); // #25f078
pub const COLOR_PRIMARY_DARK: Color32 = Color32::from_rgb(20, 170, 82);   // #14aa52
pub const COLOR_PRIMARY_BG: Color32 = Color32::from_rgba_premultiplied(27, 217, 106, 25);

// Mod Loaders & State Colors
pub const COLOR_PURPLE: Color32 = Color32::from_rgb(168, 85, 247);      // #a855f7 (Fabric Purple)
pub const COLOR_PURPLE_BG: Color32 = Color32::from_rgba_premultiplied(168, 85, 247, 28);
pub const COLOR_ORANGE: Color32 = Color32::from_rgb(249, 115, 22);       // #f97316 (Forge Orange)
pub const COLOR_ORANGE_BG: Color32 = Color32::from_rgba_premultiplied(249, 115, 22, 28);
pub const COLOR_BLUE: Color32 = Color32::from_rgb(59, 130, 246);        // #3b82f6 (NeoForge / Info)
pub const COLOR_BLUE_BG: Color32 = Color32::from_rgba_premultiplied(59, 130, 246, 28);
pub const COLOR_RED: Color32 = Color32::from_rgb(239, 68, 68);          // #ef4444 (Danger / Kill)
pub const COLOR_RED_BG: Color32 = Color32::from_rgba_premultiplied(239, 68, 68, 28);

// Typography
pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 255, 255);
pub const COLOR_TEXT_BODY: Color32 = Color32::from_rgb(236, 238, 242);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(148, 155, 172);
pub const COLOR_TEXT_SUBTLE: Color32 = Color32::from_rgb(98, 105, 122);

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
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(10, 24, 15));

    visuals.widgets.open.bg_fill = COLOR_BG_SURFACE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, COLOR_PRIMARY);
    visuals.widgets.open.rounding = Rounding::same(8.0);

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(27, 217, 106, 50);
    visuals.selection.stroke = Stroke::new(1.0_f32, COLOR_PRIMARY);

    let mut style = Style::clone(&ctx.style());
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = Margin::same(16.0);

    ctx.set_style(style);
}

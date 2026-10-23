//! Shared visual identity for the Classic GUI: one dark palette and accent
//! colour so every page reads as a single program.

use eframe::egui;

/// freeRGB accent (the blue of the logo ring; also the effects' default).
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0, 155, 222);
/// Healthy/connected accent.
pub const GOOD: egui::Color32 = egui::Color32::from_rgb(80, 200, 120);
/// Advisory warning accent (conflicts, unverified states).
pub const WARN: egui::Color32 = egui::Color32::from_rgb(240, 180, 60);
/// Error accent (unreachable, failed calls).
pub const BAD: egui::Color32 = egui::Color32::from_rgb(230, 90, 90);
/// Translucent accent fill for selected areas.
pub const SELECTION: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 155, 222, 70);

/// Installs the shared style on the egui context (call once on startup).
pub fn install(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(27, 29, 34);
    visuals.window_fill = egui::Color32::from_rgb(32, 34, 40);
    visuals.extreme_bg_color = egui::Color32::from_rgb(18, 20, 24);
    visuals.faint_bg_color = egui::Color32::from_rgb(38, 41, 48);
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = SELECTION;
    visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    let radius = egui::CornerRadius::same(6);
    for state in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        state.corner_radius = radius;
    }
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.2, ACCENT.gamma_multiply(0.75));
    visuals.widgets.hovered.expansion = 2.0;
    let mut style = egui::Style {
        visuals,
        ..Default::default()
    };
    style.spacing.item_spacing = egui::vec2(10.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 7.0);
    style.spacing.scroll_bar_width = 10.0;
    ctx.set_style(style);
}
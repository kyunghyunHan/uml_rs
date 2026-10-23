//! Shared metrics and the light desktop editor theme.
use eframe::egui::{self, Color32, Stroke};

pub const CONTROL_HEIGHT: f32 = 32.;
pub const ICON_SIZE: f32 = 26.;
pub const GAP: f32 = 8.;
pub const TEXT: Color32 = Color32::from_rgb(48, 57, 71);
pub const MUTED: Color32 = Color32::from_rgb(111, 121, 136);
pub const PANEL: Color32 = Color32::from_rgb(244, 246, 249);
pub const CANVAS: Color32 = Color32::from_rgb(251, 252, 254);
pub const BORDER: Color32 = Color32::from_rgb(221, 226, 233);
pub const HOVER: Color32 = Color32::from_rgb(232, 237, 244);
pub const BLUE: Color32 = Color32::from_rgb(55, 105, 195);
pub const BLUE_LIGHT: Color32 = Color32::from_rgb(225, 236, 253);
pub const BLUE_TEXT: Color32 = Color32::from_rgb(35, 76, 145);
pub const ERROR: Color32 = Color32::from_rgb(174, 57, 57);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::light();
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(16.));
    style.spacing.interact_size.y = CONTROL_HEIGHT;
    style.spacing.button_padding = egui::vec2(10., 7.);
    style.spacing.item_spacing = egui::vec2(GAP, GAP);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.text_edit_width = 220.;
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = Color32::WHITE;
    style.visuals.text_edit_bg_color = Some(Color32::WHITE);
    style.visuals.weak_text_color = Some(MUTED);
    style.visuals.selection.bg_fill = BLUE_LIGHT;
    style.visuals.selection.stroke = Stroke::new(1_f32, BLUE_TEXT);
    style.visuals.hyperlink_color = BLUE;
    style.visuals.error_fg_color = ERROR;
    style.visuals.window_corner_radius = 6.into();
    style.visuals.menu_corner_radius = 4.into();
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
    for (widget, fill) in [
        (&mut style.visuals.widgets.noninteractive, PANEL),
        (
            &mut style.visuals.widgets.inactive,
            Color32::from_rgb(250, 251, 253),
        ),
        (&mut style.visuals.widgets.hovered, HOVER),
        (&mut style.visuals.widgets.active, BLUE_LIGHT),
        (&mut style.visuals.widgets.open, HOVER),
    ] {
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke = Stroke::new(1_f32, BORDER);
        widget.fg_stroke = Stroke::new(1_f32, TEXT);
        widget.corner_radius = 4.into();
        widget.expansion = 0.;
    }
    ctx.set_style(style);
}

pub fn toolbar_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .inner_margin(egui::Margin::symmetric(12, 6))
        .stroke(Stroke::new(1_f32, BORDER))
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::new().fill(PANEL).inner_margin(12)
}

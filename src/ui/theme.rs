use eframe::egui;

pub fn install_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(234, 228, 214));
    visuals.panel_fill = egui::Color32::from_rgb(14, 14, 15);
    visuals.window_fill = egui::Color32::from_rgb(20, 18, 17);
    visuals.faint_bg_color = egui::Color32::from_rgb(28, 25, 23);
    visuals.extreme_bg_color = egui::Color32::from_rgb(8, 8, 9);
    visuals.selection.bg_fill = egui::Color32::from_rgb(157, 111, 58);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(240, 215, 177));
    visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    visuals.window_rounding = egui::Rounding::same(12.0);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(34, 30, 28);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(44, 37, 34);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(63, 50, 40);
    visuals.hyperlink_color = egui::Color32::from_rgb(234, 193, 133);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(13.0, 8.0);
    style.spacing.indent = 18.0;
    style.spacing.slider_width = 220.0;
    style.visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(65, 54, 46));
    style.visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(210, 197, 175));
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(28.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(18.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(15.0, egui::FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(17.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    ctx.set_style(style);
}

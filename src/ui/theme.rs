use eframe::egui;

pub fn install_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(227, 225, 220));
    visuals.panel_fill = egui::Color32::from_rgb(22, 23, 25);
    visuals.window_fill = egui::Color32::from_rgb(28, 29, 32);
    visuals.faint_bg_color = egui::Color32::from_rgb(33, 35, 38);
    visuals.extreme_bg_color = egui::Color32::from_rgb(16, 17, 19);
    visuals.selection.bg_fill = egui::Color32::from_rgb(82, 102, 132);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(168, 187, 214));
    visuals.window_rounding = egui::Rounding::same(14.0);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 52, 56));
    visuals.widgets.inactive.rounding = egui::Rounding::same(10.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(10.0);
    visuals.widgets.active.rounding = egui::Rounding::same(10.0);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(36, 38, 41);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 48, 53);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(56, 60, 67);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(53, 56, 61));
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(84, 89, 97));
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(108, 118, 131));
    visuals.hyperlink_color = egui::Color32::from_rgb(161, 186, 218);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 11.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    style.spacing.indent = 16.0;
    style.spacing.slider_width = 240.0;
    style.spacing.menu_margin = egui::Margin::symmetric(10.0, 10.0);
    style.visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 61, 66));
    style.visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(185, 189, 197));
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(30.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(17.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(15.0, egui::FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
    );
    ctx.set_style(style);
}

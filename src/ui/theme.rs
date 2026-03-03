use eframe::egui;

pub fn install_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(225, 228, 238));
    visuals.panel_fill = egui::Color32::from_rgb(14, 16, 20);
    visuals.window_fill = egui::Color32::from_rgb(18, 20, 25);
    visuals.faint_bg_color = egui::Color32::from_rgb(24, 27, 34);
    visuals.extreme_bg_color = egui::Color32::from_rgb(9, 10, 14);
    visuals.selection.bg_fill = egui::Color32::from_rgb(70, 110, 190);
    visuals.widgets.inactive.rounding = egui::Rounding::same(7.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(7.0);
    visuals.widgets.active.rounding = egui::Rounding::same(7.0);
    visuals.window_rounding = egui::Rounding::same(10.0);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(20.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(15.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(14.0, egui::FontFamily::Monospace),
    );
    ctx.set_style(style);
}


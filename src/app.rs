use std::time::{Duration, Instant};

use chrono::Local;
use eframe::egui;
use egui::text::{LayoutJob, TextFormat};

use crate::config::AppConfig;
use crate::db::Database;
use crate::services::{EditorBuffer, NoteService};

pub struct NeteApp {
    runtime: Option<Runtime>,
    fatal_error: Option<String>,
}

struct Runtime {
    config: AppConfig,
    db: Database,
    notes: Vec<crate::models::NoteSummary>,
    selected_note_id: Option<i64>,
    buffer: Option<EditorBuffer>,
    note_service: NoteService,
    sidebar_collapsed: bool,
    show_settings: bool,
    plain_text_mode: bool,
    settings_notice: Option<String>,
    last_autosave: Instant,
}

impl NeteApp {
    pub fn boot() -> Self {
        match Self::boot_runtime() {
            Ok(runtime) => Self {
                runtime: Some(runtime),
                fatal_error: None,
            },
            Err(err) => Self {
                runtime: None,
                fatal_error: Some(err.to_string()),
            },
        }
    }

    fn boot_runtime() -> crate::error::AppResult<Runtime> {
        let config = AppConfig::load_or_default()?;
        let db = Database::open(&config.db_path)?;
        let note_service = NoteService::new();

        let mut notes = db.list_notes()?;
        if notes.is_empty() {
            let id = note_service.create_note(&db, "Welcome to Nete")?;
            db.save_note(
                id,
                "Welcome to Nete",
                "# Nete\n\nA local-first markdown notebook.\n\n- Create notes\n- Link notes using [[Note Title]]\n- Add tags in the tag field\n",
            )?;
            db.set_note_tags(id, &["welcome".into(), "nete".into()])?;
            notes = db.list_notes()?;
        }

        let selected_note_id = notes.first().map(|n| n.id);
        let buffer = if let Some(id) = selected_note_id {
            note_service.open_buffer(&db, id)?
        } else {
            None
        };

        Ok(Runtime {
            config,
            db,
            notes,
            selected_note_id,
            buffer,
            note_service,
            sidebar_collapsed: false,
            show_settings: false,
            plain_text_mode: false,
            settings_notice: None,
            last_autosave: Instant::now(),
        })
    }
}

fn markdown_live_layout_job(ui: &egui::Ui, string: &str, wrap_width: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let body_color = ui.visuals().text_color();
    let muted = egui::Color32::from_rgb(132, 139, 154);
    let accent = egui::Color32::from_rgb(172, 191, 222);

    for segment in string.split_inclusive('\n') {
        let trimmed = segment.trim_start();
        let format = if trimmed.starts_with("#### ") {
            TextFormat {
                font_id: egui::FontId::proportional(20.0),
                color: body_color,
                ..Default::default()
            }
        } else if trimmed.starts_with("### ") {
            TextFormat {
                font_id: egui::FontId::proportional(23.0),
                color: body_color,
                ..Default::default()
            }
        } else if trimmed.starts_with("## ") {
            TextFormat {
                font_id: egui::FontId::proportional(27.0),
                color: body_color,
                ..Default::default()
            }
        } else if trimmed.starts_with("# ") {
            TextFormat {
                font_id: egui::FontId::proportional(32.0),
                color: accent,
                ..Default::default()
            }
        } else if trimmed.starts_with("> ") {
            TextFormat {
                font_id: egui::FontId::proportional(18.0),
                color: muted,
                italics: true,
                ..Default::default()
            }
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            TextFormat {
                font_id: egui::FontId::proportional(18.0),
                color: body_color,
                ..Default::default()
            }
        } else if trimmed.starts_with("```") {
            TextFormat {
                font_id: egui::FontId::monospace(15.0),
                color: muted,
                ..Default::default()
            }
        } else {
            TextFormat {
                font_id: egui::FontId::proportional(18.0),
                color: body_color,
                ..Default::default()
            }
        };
        job.append(segment, 0.0, format);
    }

    job.wrap.max_width = wrap_width;
    job
}

fn format_note_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.with_timezone(&Local).format("%d %b %H:%M").to_string()
}

impl Runtime {
    fn refresh_notes(&mut self) {
        if let Ok(notes) = self.db.list_notes() {
            self.notes = notes;
        }
    }

    fn open_note(&mut self, note_id: i64) {
        if let Ok(Some(buffer)) = self.note_service.open_buffer(&self.db, note_id) {
            self.selected_note_id = Some(note_id);
            self.buffer = Some(buffer);
        }
    }

    fn create_note(&mut self) {
        if let Ok(id) = self.note_service.create_note(&self.db, "") {
            self.refresh_notes();
            self.open_note(id);
        }
    }

    fn save_current(&mut self) {
        let mut saved_note_id = None;
        if let Some(buffer) = self.buffer.as_mut() {
            if self.note_service.save_buffer(&self.db, buffer).is_ok() {
                saved_note_id = Some(buffer.note_id);
            }
        }

        if let Some(note_id) = saved_note_id {
            self.refresh_notes();
            let _ = note_id;
            self.last_autosave = Instant::now();
        }
    }

    fn save_config(&mut self) {
        match AppConfig::config_path().and_then(|path| self.config.save(&path)) {
            Ok(()) => {
                self.settings_notice = Some("Settings saved".into());
            }
            Err(err) => {
                self.settings_notice = Some(format!("Failed to save settings: {err}"));
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let save = ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command);
        let new_note = ctx.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.command);
        if save {
            self.save_current();
        }
        if new_note {
            self.create_note();
        }
    }

    fn autosave_if_due(&mut self) {
        let dirty = self.buffer.as_ref().map(|b| b.dirty).unwrap_or(false);
        if !dirty {
            return;
        }
        if self.last_autosave.elapsed() >= Duration::from_millis(self.config.autosave_interval_ms) {
            self.save_current();
        }
    }

    fn ui_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_sized([28.0, 28.0], egui::Button::new(egui::RichText::new("S").size(13.0)))
                .clicked()
            {
                self.save_current();
            }
            if ui
                .add_sized([28.0, 28.0], egui::Button::new(egui::RichText::new("T").size(13.0)))
                .clicked()
            {
                self.show_settings = true;
            }

            let collapse_text = if self.sidebar_collapsed { "›" } else { "‹" };
            if ui
                .add_sized(
                    [28.0, 28.0],
                    egui::Button::new(egui::RichText::new(collapse_text).size(13.0)),
                )
                .clicked()
            {
                self.sidebar_collapsed = !self.sidebar_collapsed;
            }
            if ui
                .add_sized([28.0, 28.0], egui::Button::new(egui::RichText::new("+").size(13.0)))
                .clicked()
            {
                self.create_note();
            }
        });

        if self.sidebar_collapsed {
            return;
        }

        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(format!("{} notes", self.notes.len()))
                .small()
                .color(egui::Color32::from_gray(142)),
        );
        ui.separator();

        let mut clicked_id: Option<i64> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for note in &self.notes {
                let id = note.id;
                let selected = self.selected_note_id == Some(id);
                let title = if note.title.trim().is_empty() {
                    "Untitled"
                } else {
                    note.title.as_str()
                };

                let fill = if selected {
                    egui::Color32::from_rgb(58, 64, 75)
                } else {
                    egui::Color32::from_rgb(33, 35, 38)
                };

                egui::Frame::none()
                    .fill(fill)
                    .rounding(egui::Rounding::same(10.0))
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        let response = ui.add_sized(
                            [ui.available_width(), 22.0],
                            egui::SelectableLabel::new(
                                selected,
                                egui::RichText::new(title).size(16.5).strong(),
                            ),
                        );
                        if response.clicked() {
                            clicked_id = Some(id);
                        }

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Created").small().color(egui::Color32::from_gray(154)));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format_note_timestamp(note.created_at))
                                            .small()
                                            .color(egui::Color32::from_gray(154)),
                                    );
                                },
                            );
                        });
                    });
                ui.add_space(6.0);
            }
        });

        if let Some(id) = clicked_id {
            self.open_note(id);
        }
    }

    fn ui_settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }

        let mut open = self.show_settings;
        egui::Window::new("Settings")
            .open(&mut open)
            .default_width(420.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Writing");

                let mut changed = false;
                changed |= ui
                    .add(
                        egui::Slider::new(&mut self.config.autosave_interval_ms, 500..=10_000)
                            .text("Autosave interval (ms)"),
                    )
                    .changed();

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Cloud Sync")
                        .small()
                        .color(egui::Color32::from_gray(160)),
                );
                changed |= ui
                    .checkbox(&mut self.config.cloud_sync.enabled, "Enable cloud sync")
                    .changed();

                changed |= ui
                    .checkbox(&mut self.plain_text_mode, "Always show editor as plain text")
                    .changed();

                if changed {
                    self.save_config();
                }

                ui.add_space(8.0);
                if let Some(message) = &self.settings_notice {
                    ui.label(egui::RichText::new(message).small());
                }
            });

        self.show_settings = open;
    }

    fn ui_editor(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);

        let Some(buffer) = self.buffer.as_mut() else {
            ui.label(
                egui::RichText::new("Select or create a page to start writing")
                    .color(egui::Color32::from_gray(160)),
            );
            return;
        };

        egui::Frame::none()
            .fill(egui::Color32::from_rgb(27, 28, 31))
            .rounding(egui::Rounding::same(14.0))
            .inner_margin(egui::Margin::symmetric(18.0, 16.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Title")
                            .small()
                            .color(egui::Color32::from_gray(150)),
                    );
                    let title_changed = ui
                        .add_sized(
                            [ui.available_width(), 24.0],
                            egui::TextEdit::singleline(&mut buffer.title)
                                .hint_text("Untitled")
                                .font(egui::TextStyle::Body),
                        )
                        .changed();
                    if title_changed {
                        buffer.dirty = true;
                    }
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(10.0);

                let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
                    let job = markdown_live_layout_job(ui, text, wrap_width);
                    ui.fonts(|fonts| fonts.layout_job(job))
                };

                let mut editor = egui::TextEdit::multiline(&mut buffer.content)
                    .desired_rows(32)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY)
                    .frame(false);
                if !self.plain_text_mode {
                    editor = editor.layouter(&mut layouter);
                }
                let editor_changed = ui.add(editor).changed();

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                let words = buffer.content.split_whitespace().count();
                let save_state = if buffer.dirty { "Unsaved changes" } else { "Synced" };
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{words} words"))
                            .small()
                            .color(egui::Color32::from_gray(145)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(save_state)
                                .small()
                                .color(egui::Color32::from_gray(145)),
                        );
                    });
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Tags")
                            .small()
                            .color(egui::Color32::from_gray(150)),
                    );
                    let tags_changed = ui
                        .add_sized(
                            [ui.available_width(), 24.0],
                            egui::TextEdit::singleline(&mut buffer.tags)
                                .hint_text("notes, writing, project"),
                        )
                        .changed();
                    if tags_changed {
                        buffer.dirty = true;
                    }
                });

                if editor_changed {
                    buffer.dirty = true;
                }
            });
    }
}

impl eframe::App for NeteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(err) = &self.fatal_error {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Nete failed to start");
                ui.label(err);
            });
            return;
        }

        let Some(runtime) = self.runtime.as_mut() else {
            return;
        };

        runtime.handle_shortcuts(ctx);
        runtime.autosave_if_due();
        let repaint_delay = if runtime.buffer.as_ref().map(|b| b.dirty).unwrap_or(false) {
            Duration::from_millis(250)
        } else {
            Duration::from_millis(1000)
        };
        ctx.request_repaint_after(repaint_delay);

        egui::SidePanel::left("nav")
            .resizable(true)
            .default_width(if runtime.sidebar_collapsed { 56.0 } else { 300.0 })
            .show(ctx, |ui| runtime.ui_sidebar(ui));

        egui::CentralPanel::default().show(ctx, |ui| runtime.ui_editor(ui));
        runtime.ui_settings_window(ctx);
    }
}

use std::time::{Duration, Instant};

use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

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
    markdown_cache: CommonMarkCache,
    sidebar_collapsed: bool,
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
            markdown_cache: CommonMarkCache::default(),
            sidebar_collapsed: false,
            last_autosave: Instant::now(),
        })
    }
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
            if ui.button(if self.sidebar_collapsed { "⟩" } else { "⟨" }).clicked() {
                self.sidebar_collapsed = !self.sidebar_collapsed;
            }

            if !self.sidebar_collapsed {
                ui.heading("Nete");
            }
        });

        if self.sidebar_collapsed {
            ui.add_space(6.0);
            if ui.button("＋").clicked() {
                self.create_note();
            }
            return;
        }

        if ui.button("＋ New note").clicked() {
            self.create_note();
        }
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let items: Vec<(i64, String)> = self
                .notes
                .iter()
                .map(|n| (n.id, n.title.clone()))
                .collect();
            for (id, title) in items {
                let selected = self.selected_note_id == Some(id);
                if ui.selectable_label(selected, title).clicked() {
                    self.open_note(id);
                }
            }
        });
    }

    fn ui_editor(&mut self, ui: &mut egui::Ui) {
        let Some(buffer) = self.buffer.as_mut() else {
            ui.label("No note selected");
            return;
        };

        let title_changed = ui
            .add(egui::TextEdit::singleline(&mut buffer.title).hint_text("Title"))
            .changed();
        ui.add_space(8.0);
        ui.columns(2, |columns| {
            let editor_changed = columns[0]
                .add(
                    egui::TextEdit::multiline(&mut buffer.content)
                        .desired_rows(30)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY),
                )
                .changed();

            columns[1].heading("Preview");
            egui::ScrollArea::vertical().show(&mut columns[1], |ui| {
                CommonMarkViewer::new("note_preview").show(ui, &mut self.markdown_cache, &buffer.content);
            });

            if editor_changed {
                buffer.dirty = true;
            }
        });

        if title_changed {
            buffer.dirty = true;
        }
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
        ctx.request_repaint_after(Duration::from_millis(120));

        egui::SidePanel::left("nav")
            .resizable(true)
            .default_width(if runtime.sidebar_collapsed { 48.0 } else { 280.0 })
            .show(ctx, |ui| runtime.ui_sidebar(ui));

        egui::CentralPanel::default().show(ctx, |ui| runtime.ui_editor(ui));
    }
}

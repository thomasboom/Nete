use std::time::{Duration, Instant};

use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::config::AppConfig;
use crate::db::Database;
use crate::plugins::{PluginContext, PluginEvent, PluginManager};
use crate::services::{EditorBuffer, NoteService, SyncState};

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
    search_query: String,
    search_results: Vec<crate::models::SearchResult>,
    plugins: PluginManager,
    plugin_context: PluginContext,
    sync: SyncState,
    status: String,
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
        std::fs::create_dir_all(&config.plugin_dir)?;
        let db = Database::open(&config.db_path)?;
        let note_service = NoteService::new();

        let mut plugins = PluginManager::discover(&config.plugin_dir)?;
        let plugin_context = PluginContext {
            app_name: "Nete".into(),
            local_data_authoritative: true,
        };
        plugins.dispatch(&plugin_context, PluginEvent::AppStarted);

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
            search_query: String::new(),
            search_results: Vec::new(),
            plugins,
            plugin_context,
            sync: SyncState::disabled(),
            status: "Ready".into(),
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
            self.plugins
                .dispatch(&self.plugin_context, PluginEvent::NoteOpened(note_id));
        }
    }

    fn create_note(&mut self) {
        if let Ok(id) = self.note_service.create_note(&self.db, "") {
            self.refresh_notes();
            self.open_note(id);
            self.status = "New note created".into();
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
            self.status = format!(
                "Saved at {}",
                chrono::Local::now().format("%H:%M:%S")
            );
            self.plugins
                .dispatch(&self.plugin_context, PluginEvent::NoteSaved(note_id));
            self.last_autosave = Instant::now();
        }
    }

    fn run_search(&mut self) {
        match self
            .note_service
            .search(&self.db, &self.search_query, &self.notes)
        {
            Ok(results) => {
                self.search_results = results;
                self.plugins.dispatch(
                    &self.plugin_context,
                    PluginEvent::SearchPerformed(self.search_query.clone()),
                );
            }
            Err(err) => {
                self.status = format!("Search failed: {err}");
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

    fn ui_left_nav(&mut self, ui: &mut egui::Ui) {
        ui.heading("Nete");
        if ui.button("＋ New note").clicked() {
            self.create_note();
        }

        ui.add_space(8.0);
        let changed = ui
            .add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Search (Ctrl+K style)")
                    .desired_width(f32::INFINITY),
            )
            .changed();
        if changed {
            self.run_search();
        }

        ui.separator();
        ui.label("Notes");

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.search_query.trim().is_empty() {
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
            } else {
                let items: Vec<(i64, String)> = self
                    .search_results
                    .iter()
                    .map(|r| (r.note_id, format!("{}  ·  {}", r.title, r.snippet)))
                    .collect();
                for (id, label) in items {
                    if ui.selectable_label(self.selected_note_id == Some(id), label).clicked() {
                        self.open_note(id);
                    }
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
        let tags_changed = ui
            .add(
                egui::TextEdit::singleline(&mut buffer.tags)
                    .hint_text("tags, comma-separated")
                    .desired_width(f32::INFINITY),
            )
            .changed();

        ui.add_space(6.0);
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

        if title_changed || tags_changed {
            buffer.dirty = true;
        }
    }

    fn ui_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Context");
        if let Some(id) = self.selected_note_id {
            ui.label("Backlinks");
            if let Ok(backlinks) = self.db.backlinks_to(id) {
                for n in backlinks {
                    if ui.link(n.title).clicked() {
                        self.open_note(n.id);
                    }
                }
            }

            ui.separator();
            ui.label("Tags");
            if let Ok(tags) = self.db.note_tags(id) {
                ui.horizontal_wrapped(|ui| {
                    for tag in tags {
                        ui.label(format!("#{}", tag.name));
                    }
                });
            }
        }

        ui.separator();
        ui.label("Graph");
        self.ui_graph(ui);

        ui.separator();
        ui.label("Plugins");
        for plugin in self.plugins.plugins() {
            ui.label(format!("{} v{}", plugin.manifest.name, plugin.manifest.version));
        }
    }

    fn ui_graph(&mut self, ui: &mut egui::Ui) {
        let desired_size = egui::vec2(ui.available_width(), 220.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 8.0, egui::Color32::from_gray(18));

        let Ok(edges) = self.db.graph_edges() else {
            return;
        };
        let node_ids: Vec<i64> = self.notes.iter().map(|n| n.id).collect();
        if node_ids.is_empty() {
            return;
        }

        let center = rect.center();
        let radius = (rect.width().min(rect.height()) * 0.35).max(60.0);
        let mut positions = std::collections::HashMap::new();
        for (i, id) in node_ids.iter().enumerate() {
            let angle = (i as f32 / node_ids.len() as f32) * std::f32::consts::TAU;
            positions.insert(
                *id,
                egui::pos2(center.x + angle.cos() * radius, center.y + angle.sin() * radius),
            );
        }

        for edge in edges {
            if let (Some(a), Some(b)) = (
                positions.get(&edge.source_note_id),
                positions.get(&edge.target_note_id),
            ) {
                painter.line_segment([*a, *b], egui::Stroke::new(1.0, egui::Color32::from_gray(90)));
            }
        }

        for note in &self.notes {
            if let Some(pos) = positions.get(&note.id) {
                let selected = self.selected_note_id == Some(note.id);
                painter.circle_filled(
                    *pos,
                    if selected { 8.0 } else { 6.0 },
                    if selected {
                        egui::Color32::from_rgb(120, 170, 255)
                    } else {
                        egui::Color32::from_rgb(120, 120, 130)
                    },
                );
            }
        }

        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                for (id, pos) in positions {
                    if pos.distance(pointer) < 12.0 {
                        self.open_note(id);
                        break;
                    }
                }
            }
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
        runtime.sync.tick();
        ctx.request_repaint_after(Duration::from_millis(120));

        egui::TopBottomPanel::bottom("status_bar")
            .resizable(false)
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(&runtime.status);
                    ui.separator();
                    ui.label(&runtime.sync.last_status);
                    ui.separator();
                    ui.label("Shortcuts: Ctrl+N new · Ctrl+S save");
                });
            });

        egui::SidePanel::left("nav")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| runtime.ui_left_nav(ui));

        egui::SidePanel::right("context")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| runtime.ui_right_panel(ui));

        egui::CentralPanel::default().show(ctx, |ui| runtime.ui_editor(ui));
    }
}

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use adw::{Application, ApplicationWindow, ColorScheme, HeaderBar, StyleManager};
use chrono::Local;
use gtk::glib;
use gtk::{
    Align, Box as GtkBox, Button, ComboBoxText, Dialog, Entry, FileChooserAction,
    FileChooserNative, Label, ListBox, ListBoxRow, Orientation, Overlay, PolicyType, Revealer,
    ScrolledWindow, SelectionMode, TextBuffer, TextView,
};
use serde::{Deserialize, Serialize};

const APP_ID: &str = "local.nete.notes";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Language {
    English,
    Dutch,
}

impl Language {
    fn from_id(id: Option<glib::GString>) -> Self {
        match id.as_deref() {
            Some("nl") => Self::Dutch,
            _ => Self::English,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Dutch => "nl",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum ThemeMode {
    System,
    Light,
    Dark,
}

impl ThemeMode {
    fn from_id(id: Option<glib::GString>) -> Self {
        match id.as_deref() {
            Some("light") => Self::Light,
            Some("dark") => Self::Dark,
            _ => Self::System,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AppSettings {
    language: Language,
    theme: ThemeMode,
    notes_dir: PathBuf,
}

impl Default for AppSettings {
    fn default() -> Self {
        let notes_dir = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("NeteNotes");

        Self {
            language: Language::English,
            theme: ThemeMode::System,
            notes_dir,
        }
    }
}

#[derive(Clone)]
struct UiRefs {
    window: ApplicationWindow,
    header_title: Label,
    sidebar_revealer: Revealer,
    notes_list: ListBox,
    editor_buffer: TextBuffer,
    new_btn: Button,
    collapse_btn: Button,
    expand_btn: Button,
    settings_btn: Button,
}

#[derive(Default)]
struct AppState {
    settings: AppSettings,
    current_note: Option<PathBuf>,
    dirty: bool,
    loading_note: bool,
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Nete")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

fn load_settings() -> AppSettings {
    let path = settings_path();
    let Ok(content) = fs::read_to_string(path) else {
        return AppSettings::default();
    };

    toml::from_str(&content).unwrap_or_else(|_| AppSettings::default())
}

fn save_settings(settings: &AppSettings) {
    let cfg_dir = config_dir();
    let _ = fs::create_dir_all(&cfg_dir);
    let path = settings_path();
    if let Ok(serialized) = toml::to_string_pretty(settings) {
        let _ = fs::write(path, serialized);
    }
}

fn ensure_notes_dir(path: &Path) {
    let _ = fs::create_dir_all(path);
}

fn apply_theme(theme: ThemeMode) {
    let style = StyleManager::default();
    match theme {
        ThemeMode::System => style.set_color_scheme(ColorScheme::Default),
        ThemeMode::Light => style.set_color_scheme(ColorScheme::ForceLight),
        ThemeMode::Dark => style.set_color_scheme(ColorScheme::ForceDark),
    };
}

fn text_for(lang: Language, key: &str) -> &'static str {
    match (lang, key) {
        (Language::English, "title") => "Nete Notes",
        (Language::English, "new_note") => "Create Note",
        (Language::English, "toggle_sidebar") => "Collapse/Expand Sidebar",
        (Language::English, "expand_sidebar") => "Expand Sidebar",
        (Language::English, "settings") => "Settings",
        (Language::English, "settings_title") => "Settings",
        (Language::English, "language") => "Language",
        (Language::English, "theme") => "Theme",
        (Language::English, "notes_path") => "Notes Folder",
        (Language::English, "choose_path") => "Choose Folder",
        (Language::English, "theme_system") => "System",
        (Language::English, "theme_light") => "Light",
        (Language::English, "theme_dark") => "Dark",
        (Language::Dutch, "title") => "Nete Notities",
        (Language::Dutch, "new_note") => "Notitie Maken",
        (Language::Dutch, "toggle_sidebar") => "Zijbalk In-/Uitklappen",
        (Language::Dutch, "expand_sidebar") => "Zijbalk Uitklappen",
        (Language::Dutch, "settings") => "Instellingen",
        (Language::Dutch, "settings_title") => "Instellingen",
        (Language::Dutch, "language") => "Taal",
        (Language::Dutch, "theme") => "Thema",
        (Language::Dutch, "notes_path") => "Notitiemap",
        (Language::Dutch, "choose_path") => "Map Kiezen",
        (Language::Dutch, "theme_system") => "Systeem",
        (Language::Dutch, "theme_light") => "Licht",
        (Language::Dutch, "theme_dark") => "Donker",
        _ => "",
    }
}

fn update_translations(ui: &UiRefs, language: Language) {
    ui.window.set_title(Some(text_for(language, "title")));
    ui.header_title.set_text(text_for(language, "title"));
    ui.new_btn
        .set_tooltip_text(Some(text_for(language, "new_note")));
    ui.collapse_btn
        .set_tooltip_text(Some(text_for(language, "toggle_sidebar")));
    ui.expand_btn
        .set_tooltip_text(Some(text_for(language, "expand_sidebar")));
    ui.settings_btn
        .set_tooltip_text(Some(text_for(language, "settings")));
}

fn note_title_from_markdown(content: &str, fallback: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cleaned = trimmed.trim_start_matches('#').trim();
        if !cleaned.is_empty() {
            return cleaned.chars().take(48).collect();
        }
    }
    fallback.to_owned()
}

fn clear_listbox(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn list_markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = vec![];
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }
    files.sort_by(|a, b| {
        let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
        let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });
    files
}

fn repopulate_notes_list(ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
    clear_listbox(&ui.notes_list);

    let note_paths = {
        let st = state.borrow();
        list_markdown_files(&st.settings.notes_dir)
    };

    for path in note_paths {
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("note.md")
            .to_string();
        let title = fs::read_to_string(&path)
            .map(|txt| note_title_from_markdown(&txt, &filename))
            .unwrap_or(filename);

        let row = ListBoxRow::new();
        row.set_selectable(true);
        row.set_activatable(true);
        row.set_widget_name(path.to_string_lossy().as_ref());
        let label = Label::new(Some(&title));
        label.set_halign(Align::Start);
        label.set_xalign(0.0);
        label.add_css_class("title-4");
        row.set_child(Some(&label));
        ui.notes_list.append(&row);
    }
}

fn save_current_note(ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
    let maybe_path = state.borrow().current_note.clone();
    if let Some(path) = maybe_path {
        let text = ui
            .editor_buffer
            .text(
                &ui.editor_buffer.start_iter(),
                &ui.editor_buffer.end_iter(),
                true,
            )
            .to_string();
        if fs::write(path, text).is_ok() {
            state.borrow_mut().dirty = false;
        }
    }
}

fn load_note_into_editor(path: &Path, ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
    let content = fs::read_to_string(path).unwrap_or_default();
    {
        let mut st = state.borrow_mut();
        st.loading_note = true;
        st.current_note = Some(path.to_path_buf());
    }
    ui.editor_buffer.set_text(&content);
    {
        let mut st = state.borrow_mut();
        st.loading_note = false;
        st.dirty = false;
    }
}

fn create_new_note(ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
    let dir = state.borrow().settings.notes_dir.clone();
    ensure_notes_dir(&dir);
    let name = format!("note-{}.md", Local::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(name);
    let initial = "# New note\n";
    if fs::write(&path, initial).is_ok() {
        repopulate_notes_list(ui, state);
        load_note_into_editor(&path, ui, state);
    }
}

fn build_settings_dialog(ui: &UiRefs, state: &Rc<RefCell<AppState>>) -> Dialog {
    let st = state.borrow();
    let dialog = Dialog::builder()
        .transient_for(&ui.window)
        .modal(true)
        .title(text_for(st.settings.language, "settings_title"))
        .default_width(520)
        .default_height(220)
        .build();
    drop(st);

    let content = dialog.content_area();
    let wrapper = GtkBox::new(Orientation::Vertical, 12);
    wrapper.set_margin_start(16);
    wrapper.set_margin_end(16);
    wrapper.set_margin_top(16);
    wrapper.set_margin_bottom(16);

    let st = state.borrow();
    let lang = st.settings.language;
    let theme = st.settings.theme;
    let notes_dir = st.settings.notes_dir.to_string_lossy().to_string();
    drop(st);

    let lang_row = GtkBox::new(Orientation::Horizontal, 8);
    let lang_label = Label::new(Some(text_for(lang, "language")));
    lang_label.set_width_chars(14);
    lang_label.set_halign(Align::Start);
    lang_label.set_xalign(0.0);
    let lang_combo = ComboBoxText::new();
    lang_combo.append(Some("en"), "English");
    lang_combo.append(Some("nl"), "Nederlands");
    lang_combo.set_active_id(Some(lang.id()));
    lang_row.append(&lang_label);
    lang_row.append(&lang_combo);

    let theme_row = GtkBox::new(Orientation::Horizontal, 8);
    let theme_label = Label::new(Some(text_for(lang, "theme")));
    theme_label.set_width_chars(14);
    theme_label.set_halign(Align::Start);
    theme_label.set_xalign(0.0);
    let theme_combo = ComboBoxText::new();
    theme_combo.append(Some("system"), text_for(lang, "theme_system"));
    theme_combo.append(Some("light"), text_for(lang, "theme_light"));
    theme_combo.append(Some("dark"), text_for(lang, "theme_dark"));
    theme_combo.set_active_id(Some(theme.id()));
    theme_row.append(&theme_label);
    theme_row.append(&theme_combo);

    let path_row = GtkBox::new(Orientation::Horizontal, 8);
    let path_label = Label::new(Some(text_for(lang, "notes_path")));
    path_label.set_width_chars(14);
    path_label.set_halign(Align::Start);
    path_label.set_xalign(0.0);
    let path_entry = Entry::new();
    path_entry.set_hexpand(true);
    path_entry.set_editable(false);
    path_entry.set_text(&notes_dir);
    let choose_btn = Button::with_label(text_for(lang, "choose_path"));
    path_row.append(&path_label);
    path_row.append(&path_entry);
    path_row.append(&choose_btn);

    wrapper.append(&lang_row);
    wrapper.append(&theme_row);
    wrapper.append(&path_row);
    content.append(&wrapper);

    {
        let state = state.clone();
        let ui = ui.clone();
        let dialog_ref = dialog.clone();
        lang_combo.connect_changed(move |combo| {
            let mut st = state.borrow_mut();
            st.settings.language = Language::from_id(combo.active_id());
            save_settings(&st.settings);
            update_translations(&ui, st.settings.language);
            dialog_ref.set_title(Some(text_for(st.settings.language, "settings_title")));
        });
    }

    {
        let state = state.clone();
        theme_combo.connect_changed(move |combo| {
            let mut st = state.borrow_mut();
            st.settings.theme = ThemeMode::from_id(combo.active_id());
            apply_theme(st.settings.theme);
            save_settings(&st.settings);
        });
    }

    {
        let state = state.clone();
        let ui = ui.clone();
        let path_entry = path_entry.clone();
        let dialog_parent = dialog.clone();
        choose_btn.connect_clicked(move |_| {
            let chooser = FileChooserNative::builder()
                .title("Choose Notes Folder")
                .transient_for(&dialog_parent)
                .action(FileChooserAction::SelectFolder)
                .accept_label("Select")
                .cancel_label("Cancel")
                .build();
            let state = state.clone();
            let ui = ui.clone();
            let path_entry = path_entry.clone();
            chooser.connect_response(move |dialog, response| {
                if response == gtk::ResponseType::Accept
                    && let Some(file) = dialog.file()
                    && let Some(path) = file.path()
                {
                    {
                        let mut st = state.borrow_mut();
                        st.settings.notes_dir = path.clone();
                        save_settings(&st.settings);
                    }
                    ensure_notes_dir(&path);
                    path_entry.set_text(path.to_string_lossy().as_ref());
                    repopulate_notes_list(&ui, &state);
                }
                dialog.destroy();
            });
            chooser.show();
        });
    }

    dialog
}

fn build_ui(app: &Application) {
    let initial_settings = load_settings();
    ensure_notes_dir(&initial_settings.notes_dir);
    save_settings(&initial_settings);
    apply_theme(initial_settings.theme);

    let window = ApplicationWindow::builder()
        .application(app)
        .title(text_for(initial_settings.language, "title"))
        .default_width(1000)
        .default_height(700)
        .build();

    let header = HeaderBar::new();
    let title = Label::new(Some(text_for(initial_settings.language, "title")));
    title.add_css_class("title-3");
    header.set_title_widget(Some(&title));

    let shell = GtkBox::new(Orientation::Vertical, 0);
    shell.append(&header);

    let root = GtkBox::new(Orientation::Horizontal, 0);
    root.set_margin_start(8);
    root.set_margin_end(8);
    root.set_margin_top(8);
    root.set_margin_bottom(8);
    root.add_css_class("view");

    let sidebar_revealer = Revealer::new();
    sidebar_revealer.set_reveal_child(true);
    sidebar_revealer.set_transition_duration(200);

    let sidebar = GtkBox::new(Orientation::Vertical, 8);
    sidebar.set_margin_start(8);
    sidebar.set_margin_end(8);
    sidebar.set_margin_top(8);
    sidebar.set_margin_bottom(8);
    sidebar.set_width_request(260);
    sidebar.add_css_class("navigation-sidebar");

    let sidebar_top = GtkBox::new(Orientation::Horizontal, 8);
    let new_btn = Button::builder()
        .icon_name("document-new-symbolic")
        .tooltip_text(text_for(initial_settings.language, "new_note"))
        .build();
    let collapse_btn = Button::builder()
        .icon_name("sidebar-show-right-symbolic")
        .tooltip_text(text_for(initial_settings.language, "toggle_sidebar"))
        .build();
    let settings_btn = Button::builder()
        .icon_name("emblem-system-symbolic")
        .tooltip_text(text_for(initial_settings.language, "settings"))
        .build();
    sidebar_top.append(&new_btn);
    sidebar_top.append(&collapse_btn);
    sidebar_top.append(&settings_btn);

    let notes_list = ListBox::new();
    notes_list.set_selection_mode(SelectionMode::Single);
    notes_list.add_css_class("boxed-list");
    let notes_scroller = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .min_content_height(200)
        .vexpand(true)
        .build();
    notes_scroller.set_child(Some(&notes_list));

    sidebar.append(&sidebar_top);
    sidebar.append(&notes_scroller);
    sidebar_revealer.set_child(Some(&sidebar));

    let editor_buffer = TextBuffer::new(None::<&gtk::TextTagTable>);
    let editor = TextView::builder()
        .monospace(true)
        .buffer(&editor_buffer)
        .wrap_mode(gtk::WrapMode::WordChar)
        .vexpand(true)
        .hexpand(true)
        .build();
    let editor_scroller = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .build();
    editor_scroller.add_css_class("card");
    editor_scroller.set_margin_start(8);
    editor_scroller.set_margin_end(8);
    editor_scroller.set_margin_top(8);
    editor_scroller.set_margin_bottom(8);
    editor_scroller.set_child(Some(&editor));

    let overlay = Overlay::new();
    overlay.set_child(Some(&editor_scroller));

    let expand_btn = Button::builder()
        .icon_name("sidebar-show-left-symbolic")
        .tooltip_text(text_for(initial_settings.language, "expand_sidebar"))
        .halign(Align::Start)
        .valign(Align::Start)
        .margin_start(16)
        .margin_top(16)
        .css_classes(["circular"])
        .visible(false)
        .build();
    overlay.add_overlay(&expand_btn);

    root.append(&sidebar_revealer);
    root.append(&overlay);
    shell.append(&root);
    window.set_content(Some(&shell));

    let ui = UiRefs {
        window: window.clone(),
        header_title: title.clone(),
        sidebar_revealer: sidebar_revealer.clone(),
        notes_list: notes_list.clone(),
        editor_buffer: editor_buffer.clone(),
        new_btn: new_btn.clone(),
        collapse_btn: collapse_btn.clone(),
        expand_btn: expand_btn.clone(),
        settings_btn: settings_btn.clone(),
    };

    let state = Rc::new(RefCell::new(AppState {
        settings: initial_settings.clone(),
        ..Default::default()
    }));

    repopulate_notes_list(&ui, &state);

    {
        let ui = ui.clone();
        let state = state.clone();
        notes_list.connect_row_selected(move |_list, row| {
            if let Some(row) = row {
                if state.borrow().dirty {
                    save_current_note(&ui, &state);
                }
                let path_text = row.widget_name().to_string();
                load_note_into_editor(Path::new(&path_text), &ui, &state);
            }
        });
    }

    {
        let ui = ui.clone();
        let state = state.clone();
        new_btn.connect_clicked(move |_| {
            if state.borrow().dirty {
                save_current_note(&ui, &state);
            }
            create_new_note(&ui, &state);
        });
    }

    {
        let ui = ui.clone();
        collapse_btn.connect_clicked(move |_| {
            let visible = ui.sidebar_revealer.reveals_child();
            ui.sidebar_revealer.set_reveal_child(!visible);
            ui.expand_btn.set_visible(visible);
        });
    }

    {
        let ui = ui.clone();
        expand_btn.connect_clicked(move |_| {
            ui.sidebar_revealer.set_reveal_child(true);
            ui.expand_btn.set_visible(false);
        });
    }

    {
        let ui = ui.clone();
        let state = state.clone();
        settings_btn.connect_clicked(move |_| {
            let dialog = build_settings_dialog(&ui, &state);
            dialog.connect_close_request(|d| {
                d.hide();
                glib::Propagation::Stop
            });
            dialog.show();
        });
    }

    {
        let state = state.clone();
        editor_buffer.connect_changed(move |_| {
            let mut st = state.borrow_mut();
            if st.loading_note {
                return;
            }
            st.dirty = true;
        });
    }

    {
        let ui = ui.clone();
        let state = state.clone();
        glib::timeout_add_seconds_local(2, move || {
            if state.borrow().dirty {
                save_current_note(&ui, &state);
                repopulate_notes_list(&ui, &state);
            }
            glib::ControlFlow::Continue
        });
    }

    {
        let ui = ui.clone();
        let state = state.clone();
        window.connect_close_request(move |_| {
            if state.borrow().dirty {
                save_current_note(&ui, &state);
            }
            glib::Propagation::Proceed
        });
    }

    if let Some(first_row) = notes_list.row_at_index(0) {
        notes_list.select_row(Some(&first_row));
    } else {
        create_new_note(&ui, &state);
    }

    update_translations(&ui, initial_settings.language);
    window.present();
}

fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use adw::{
    ActionRow, Application, ApplicationWindow, Clamp, ColorScheme, ComboRow, HeaderBar,
    OverlaySplitView, PreferencesGroup, PreferencesPage, PreferencesWindow, StyleManager,
    ToolbarView,
};
use chrono::Local;
use gtk::glib;
use gtk::{
    Align, Box as GtkBox, Button, FileChooserAction, FileChooserNative, Label, ListBox, ListBoxRow,
    Orientation, PolicyType, ScrolledWindow, SelectionMode, StringList, TextBuffer, TextView,
};
use serde::{Deserialize, Serialize};

const APP_ID: &str = "local.nete.notes";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Language {
    English,
    Dutch,
}

impl Language {
    fn from_selected(index: u32) -> Self {
        match index {
            1 => Self::Dutch,
            _ => Self::English,
        }
    }

    fn selected(self) -> u32 {
        match self {
            Self::English => 0,
            Self::Dutch => 1,
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
    fn from_selected(index: u32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }

    fn selected(self) -> u32 {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
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
    split_view: OverlaySplitView,
    notes_list: ListBox,
    editor_buffer: TextBuffer,
    new_btn: Button,
    sidebar_btn: Button,
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
        (Language::English, "toggle_sidebar") => "Toggle Sidebar",
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
        (Language::Dutch, "toggle_sidebar") => "Zijbalk Tonen/Verbergen",
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
    ui.sidebar_btn
        .set_tooltip_text(Some(text_for(language, "toggle_sidebar")));
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

fn build_settings_window(ui: &UiRefs, state: &Rc<RefCell<AppState>>) -> PreferencesWindow {
    let st = state.borrow();
    let settings_window = PreferencesWindow::builder()
        .transient_for(&ui.window)
        .modal(true)
        .title(text_for(st.settings.language, "settings_title"))
        .default_width(620)
        .default_height(420)
        .search_enabled(false)
        .build();
    let lang = st.settings.language;
    let theme = st.settings.theme;
    let notes_dir = st.settings.notes_dir.to_string_lossy().to_string();
    drop(st);

    let page = PreferencesPage::new();
    let group = PreferencesGroup::new();
    group.set_title(text_for(lang, "settings_title"));

    let lang_model = StringList::new(&["English", "Nederlands"]);
    let lang_row = ComboRow::builder()
        .title(text_for(lang, "language"))
        .selected(lang.selected())
        .build();
    lang_row.set_model(Some(&lang_model));

    let theme_model = StringList::new(&[
        text_for(lang, "theme_system"),
        text_for(lang, "theme_light"),
        text_for(lang, "theme_dark"),
    ]);
    let theme_row = ComboRow::builder()
        .title(text_for(lang, "theme"))
        .selected(theme.selected())
        .build();
    theme_row.set_model(Some(&theme_model));

    let path_row = ActionRow::builder()
        .title(text_for(lang, "notes_path"))
        .subtitle(&notes_dir)
        .activatable(false)
        .build();
    let choose_btn = Button::with_label(text_for(lang, "choose_path"));
    choose_btn.add_css_class("flat");
    path_row.add_suffix(&choose_btn);
    path_row.set_activatable_widget(Some(&choose_btn));

    group.add(&lang_row);
    group.add(&theme_row);
    group.add(&path_row);
    page.add(&group);
    settings_window.add(&page);

    {
        let state = state.clone();
        let ui = ui.clone();
        let settings_window = settings_window.clone();
        let theme_row = theme_row.clone();
        let path_row = path_row.clone();
        let choose_btn = choose_btn.clone();
        lang_row.connect_selected_notify(move |row| {
            let (language, theme_selected) = {
                let mut st = state.borrow_mut();
                st.settings.language = Language::from_selected(row.selected());
                save_settings(&st.settings);
                (st.settings.language, st.settings.theme.selected())
            };

            update_translations(&ui, language);
            settings_window.set_title(Some(text_for(language, "settings_title")));
            let theme_model = StringList::new(&[
                text_for(language, "theme_system"),
                text_for(language, "theme_light"),
                text_for(language, "theme_dark"),
            ]);
            theme_row.set_model(Some(&theme_model));
            theme_row.set_selected(theme_selected);
            theme_row.set_title(text_for(language, "theme"));
            path_row.set_title(text_for(language, "notes_path"));
            choose_btn.set_label(text_for(language, "choose_path"));
        });
    }

    {
        let state = state.clone();
        theme_row.connect_selected_notify(move |row| {
            let mut st = state.borrow_mut();
            st.settings.theme = ThemeMode::from_selected(row.selected());
            apply_theme(st.settings.theme);
            save_settings(&st.settings);
        });
    }

    {
        let state = state.clone();
        let ui = ui.clone();
        let path_row = path_row.clone();
        let settings_window = settings_window.clone();
        choose_btn.connect_clicked(move |_| {
            let chooser = FileChooserNative::builder()
                .title("Choose Notes Folder")
                .transient_for(&settings_window)
                .action(FileChooserAction::SelectFolder)
                .accept_label("Select")
                .cancel_label("Cancel")
                .build();
            let state = state.clone();
            let ui = ui.clone();
            let path_row = path_row.clone();
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
                    path_row.set_subtitle(path.to_string_lossy().as_ref());
                    repopulate_notes_list(&ui, &state);
                }
                dialog.destroy();
            });
            chooser.show();
        });
    }

    settings_window
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

    let toolbar_view = ToolbarView::new();
    let split_view = OverlaySplitView::new();
    split_view.set_show_sidebar(true);
    split_view.set_pin_sidebar(true);
    split_view.set_sidebar_width_fraction(0.24);
    split_view.set_min_sidebar_width(220.0);
    split_view.set_max_sidebar_width(320.0);
    split_view.set_hexpand(true);
    split_view.set_vexpand(true);

    let sidebar = GtkBox::new(Orientation::Vertical, 8);
    sidebar.set_margin_start(8);
    sidebar.set_margin_end(8);
    sidebar.set_margin_top(8);
    sidebar.set_margin_bottom(8);
    sidebar.add_css_class("navigation-sidebar");

    let new_btn = Button::builder()
        .icon_name("document-new-symbolic")
        .tooltip_text(text_for(initial_settings.language, "new_note"))
        .build();
    new_btn.add_css_class("flat");
    let sidebar_btn = Button::builder()
        .icon_name("sidebar-show-symbolic")
        .tooltip_text(text_for(initial_settings.language, "toggle_sidebar"))
        .build();
    sidebar_btn.add_css_class("flat");
    let settings_btn = Button::builder()
        .icon_name("emblem-system-symbolic")
        .tooltip_text(text_for(initial_settings.language, "settings"))
        .build();
    settings_btn.add_css_class("flat");
    header.pack_start(&sidebar_btn);
    header.pack_end(&settings_btn);
    header.pack_end(&new_btn);

    let notes_list = ListBox::new();
    notes_list.set_selection_mode(SelectionMode::Single);
    notes_list.add_css_class("boxed-list");
    let notes_scroller = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .min_content_height(200)
        .vexpand(true)
        .build();
    notes_scroller.set_child(Some(&notes_list));

    sidebar.append(&notes_scroller);

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
    let editor_clamp = Clamp::builder()
        .maximum_size(1200)
        .tightening_threshold(600)
        .child(&editor_scroller)
        .build();

    split_view.set_sidebar(Some(&sidebar));
    split_view.set_content(Some(&editor_clamp));
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&split_view));
    window.set_content(Some(&toolbar_view));

    let ui = UiRefs {
        window: window.clone(),
        header_title: title.clone(),
        split_view: split_view.clone(),
        notes_list: notes_list.clone(),
        editor_buffer: editor_buffer.clone(),
        new_btn: new_btn.clone(),
        sidebar_btn: sidebar_btn.clone(),
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
        sidebar_btn.connect_clicked(move |_| {
            let shown = ui.split_view.shows_sidebar();
            ui.split_view.set_show_sidebar(!shown);
        });
    }

    {
        let ui = ui.clone();
        let state = state.clone();
        settings_btn.connect_clicked(move |_| {
            let settings_window = build_settings_window(&ui, &state);
            settings_window.present();
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

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use adw::{
    ActionRow, Application, ApplicationWindow, ColorScheme, ComboRow, HeaderBar, OverlaySplitView,
    PreferencesGroup, PreferencesPage, PreferencesWindow, StyleManager, ToolbarView, WindowTitle,
};
use chrono::Local;
use gtk::glib;
use gtk::{
    Align, Box as GtkBox, Button, Entry, EventControllerKey, FileChooserAction, FileChooserNative,
    Image, Label, ListBox, ListBoxRow, Orientation, Overlay, PolicyType, ScrolledWindow,
    SelectionMode, StringList, TextBuffer, TextView, TextWindowType,
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
    header_title: WindowTitle,
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

#[derive(Default)]
struct SlashMenuState {
    items: Vec<CommandMenuItem>,
    slash_offset: Option<i32>,
    replace_end_offset: Option<i32>,
    visible: bool,
    suppress_change: bool,
}

#[derive(Default)]
struct CommandPaletteState {
    items: Vec<CommandMenuItem>,
    visible: bool,
}

#[derive(Clone)]
enum CommandMenuAction {
    NoteLink(String),
    InsertText(String),
    NoOp,
    OpenNote(PathBuf),
    CreateNote,
    ToggleSidebar,
    OpenSettings,
    SetLanguage(Language),
    SetTheme(ThemeMode),
    ChooseNotesFolder,
}

impl CommandMenuAction {
    fn icon_name(&self) -> &'static str {
        match self {
            Self::NoteLink(_) => "text-x-generic-symbolic",
            Self::InsertText(_) => "applications-engineering-symbolic",
            Self::NoOp => "dialog-warning-symbolic",
            Self::OpenNote(_) => "text-x-generic-symbolic",
            Self::CreateNote => "document-new-symbolic",
            Self::ToggleSidebar => "sidebar-show-symbolic",
            Self::OpenSettings | Self::ChooseNotesFolder => "emblem-system-symbolic",
            Self::SetLanguage(_) => "preferences-desktop-locale-symbolic",
            Self::SetTheme(_) => "weather-clear-night-symbolic",
        }
    }
}

#[derive(Clone)]
struct CommandMenuItem {
    label: String,
    action: CommandMenuAction,
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

fn install_command_palette_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        .command-palette {
            background-color: alpha(@window_bg_color, 1.0);
            border-radius: 14px;
            border: 1px solid alpha(@borders, 0.7);
            box-shadow: 0 12px 30px alpha(black, 0.22);
            opacity: 1;
        }
        .command-palette entry {
            background-color: alpha(@window_bg_color, 1.0);
            border: 1px solid alpha(@borders, 0.8);
            opacity: 1;
        }
        .command-palette scrolledwindow,
        .command-palette list,
        .command-palette row {
            background-color: alpha(@window_bg_color, 1.0);
            opacity: 1;
        }
        ",
    );
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
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
        (Language::English, "choose_notes_folder") => "Choose Notes Folder",
        (Language::English, "select") => "Select",
        (Language::English, "cancel") => "Cancel",
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
        (Language::Dutch, "choose_notes_folder") => "Notitiemap Kiezen",
        (Language::Dutch, "select") => "Selecteren",
        (Language::Dutch, "cancel") => "Annuleren",
        (Language::Dutch, "theme_system") => "Systeem",
        (Language::Dutch, "theme_light") => "Licht",
        (Language::Dutch, "theme_dark") => "Donker",
        _ => "",
    }
}

fn update_translations(ui: &UiRefs, language: Language) {
    ui.window.set_title(Some(text_for(language, "title")));
    ui.header_title.set_title(text_for(language, "title"));
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

fn note_subtitle(path: &Path) -> String {
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("note.md")
        .to_string();
    let modified = fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(|mtime| {
            let dt: chrono::DateTime<Local> = mtime.into();
            dt.format("%Y-%m-%d %H:%M").to_string()
        })
        .unwrap_or_else(|| "unknown time".to_string());
    format!("{filename}  •  {modified}")
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

fn linkable_note_titles(state: &Rc<RefCell<AppState>>) -> Vec<String> {
    let (notes_dir, current_note) = {
        let st = state.borrow();
        (st.settings.notes_dir.clone(), st.current_note.clone())
    };

    let mut titles = Vec::new();
    for path in list_markdown_files(&notes_dir) {
        if current_note.as_ref() == Some(&path) {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("note.md")
            .to_string();
        let title = fs::read_to_string(&path)
            .map(|txt| note_title_from_markdown(&txt, &filename))
            .unwrap_or(filename);
        titles.push(title);
    }

    titles
}

fn slash_menu_items(state: &Rc<RefCell<AppState>>, query: &str) -> Vec<CommandMenuItem> {
    let normalized_query = query.to_lowercase();
    let mut items = vec![
        CommandMenuItem {
            label: "Header".to_string(),
            action: CommandMenuAction::InsertText("# ".to_string()),
        },
        CommandMenuItem {
            label: "Link".to_string(),
            action: CommandMenuAction::InsertText("[text](url)".to_string()),
        },
        CommandMenuItem {
            label: "Bold".to_string(),
            action: CommandMenuAction::InsertText("**bold text**".to_string()),
        },
        CommandMenuItem {
            label: "Italic".to_string(),
            action: CommandMenuAction::InsertText("*italic text*".to_string()),
        },
    ];

    items.retain(|item| item.label.to_lowercase().contains(&normalized_query));

    let note_items = linkable_note_titles(state)
        .into_iter()
        .filter(|title| title.to_lowercase().contains(&normalized_query))
        .map(|title| CommandMenuItem {
            label: title.clone(),
            action: CommandMenuAction::NoteLink(title),
        });
    items.extend(note_items);

    items
}

fn command_bar_items(state: &Rc<RefCell<AppState>>, query: &str) -> Vec<CommandMenuItem> {
    let normalized_query = query.trim().to_lowercase();
    let query_is_empty = normalized_query.is_empty();

    let mut items = vec![
        CommandMenuItem {
            label: "Create New Note".to_string(),
            action: CommandMenuAction::CreateNote,
        },
        CommandMenuItem {
            label: "Toggle Sidebar".to_string(),
            action: CommandMenuAction::ToggleSidebar,
        },
        CommandMenuItem {
            label: "Open Settings".to_string(),
            action: CommandMenuAction::OpenSettings,
        },
        CommandMenuItem {
            label: "Choose Notes Folder".to_string(),
            action: CommandMenuAction::ChooseNotesFolder,
        },
        CommandMenuItem {
            label: "Theme: System".to_string(),
            action: CommandMenuAction::SetTheme(ThemeMode::System),
        },
        CommandMenuItem {
            label: "Theme: Light".to_string(),
            action: CommandMenuAction::SetTheme(ThemeMode::Light),
        },
        CommandMenuItem {
            label: "Theme: Dark".to_string(),
            action: CommandMenuAction::SetTheme(ThemeMode::Dark),
        },
        CommandMenuItem {
            label: "Language: English".to_string(),
            action: CommandMenuAction::SetLanguage(Language::English),
        },
        CommandMenuItem {
            label: "Language: Nederlands".to_string(),
            action: CommandMenuAction::SetLanguage(Language::Dutch),
        },
        CommandMenuItem {
            label: "Insert Header".to_string(),
            action: CommandMenuAction::InsertText("# ".to_string()),
        },
        CommandMenuItem {
            label: "Insert Link".to_string(),
            action: CommandMenuAction::InsertText("[text](url)".to_string()),
        },
        CommandMenuItem {
            label: "Insert Bold".to_string(),
            action: CommandMenuAction::InsertText("**bold text**".to_string()),
        },
        CommandMenuItem {
            label: "Insert Italic".to_string(),
            action: CommandMenuAction::InsertText("*italic text*".to_string()),
        },
    ];

    if !query_is_empty {
        items.retain(|item| item.label.to_lowercase().contains(&normalized_query));
    }

    let notes_dir = state.borrow().settings.notes_dir.clone();
    let mut note_items = Vec::new();
    for path in list_markdown_files(&notes_dir) {
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("note.md")
            .to_string();
        let content = fs::read_to_string(&path).unwrap_or_default();
        let title = note_title_from_markdown(&content, &filename);
        let haystack = format!("{title}\n{filename}\n{content}").to_lowercase();
        if query_is_empty || haystack.contains(&normalized_query) {
            note_items.push(CommandMenuItem {
                label: format!("Open Note: {title}"),
                action: CommandMenuAction::OpenNote(path),
            });
        }
        if query_is_empty && note_items.len() >= 10 {
            break;
        }
    }

    items.extend(note_items);
    if items.is_empty() {
        items.push(CommandMenuItem {
            label: format!("No results for \"{}\"", query.trim()),
            action: CommandMenuAction::NoOp,
        });
    }
    items
}

fn slash_query_at_cursor(buffer: &TextBuffer) -> Option<(i32, i32, String)> {
    let insert_mark = buffer.get_insert();
    let cursor = buffer.iter_at_mark(&insert_mark);
    let cursor_offset = cursor.offset();
    let mut scan = cursor;

    while scan.backward_char() {
        let ch = scan.char();
        if ch == '/' {
            let query_start = buffer.iter_at_offset(scan.offset() + 1);
            let query = buffer.text(&query_start, &buffer.iter_at_offset(cursor_offset), true);
            return Some((scan.offset(), cursor_offset, query.to_string()));
        }
        if ch.is_whitespace() {
            break;
        }
    }

    None
}

fn position_command_menu(text_view: &TextView, menu_box: &GtkBox) {
    let buffer = text_view.buffer();
    let insert_mark = buffer.get_insert();
    let iter = buffer.iter_at_mark(&insert_mark);
    let rect = text_view.iter_location(&iter);
    let (x, y) = text_view.buffer_to_window_coords(
        TextWindowType::Widget,
        rect.x(),
        rect.y() + rect.height(),
    );
    menu_box.set_margin_start((x + 8).max(8));
    menu_box.set_margin_top((y + 8).max(8));
}

fn populate_command_list(list: &ListBox, items: &[CommandMenuItem]) {
    clear_listbox(list);
    for item_data in items {
        let row = ListBoxRow::new();
        row.set_selectable(true);
        row.set_activatable(true);

        let content = GtkBox::new(Orientation::Horizontal, 8);
        content.set_margin_top(4);
        content.set_margin_bottom(4);
        content.set_margin_start(8);
        content.set_margin_end(8);

        let label = Label::new(Some(&item_data.label));
        label.set_halign(Align::Start);
        label.set_hexpand(true);
        label.set_xalign(0.0);
        content.append(&label);

        let icon = Image::from_icon_name(item_data.action.icon_name());
        icon.add_css_class("dim-label");
        icon.set_halign(Align::End);
        content.append(&icon);

        row.set_child(Some(&content));
        list.append(&row);
    }
}

fn hide_slash_menu(menu_box: &GtkBox, menu_state: &Rc<RefCell<SlashMenuState>>) {
    menu_box.set_visible(false);
    let mut st = menu_state.borrow_mut();
    st.visible = false;
    st.slash_offset = None;
    st.replace_end_offset = None;
    st.items.clear();
}

fn hide_command_palette(menu_box: &GtkBox, menu_state: &Rc<RefCell<CommandPaletteState>>) {
    menu_box.set_visible(false);
    let mut st = menu_state.borrow_mut();
    st.visible = false;
    st.items.clear();
}

fn resize_command_palette(
    window: &ApplicationWindow,
    palette_box: &GtkBox,
    palette_scroller: &ScrolledWindow,
) {
    let width = if window.allocated_width() > 0 {
        window.allocated_width()
    } else {
        window.default_width()
    };
    let height = if window.allocated_height() > 0 {
        window.allocated_height()
    } else {
        window.default_height()
    };

    let palette_width = (width - 48).clamp(420, 1280);
    let max_list_height = (height - 180).clamp(220, 760);
    let min_list_height = (max_list_height / 2).clamp(160, 340);

    palette_box.set_size_request(palette_width, -1);
    palette_scroller.set_max_content_height(max_list_height);
    palette_scroller.set_min_content_height(min_list_height);
}

fn choose_notes_folder<W: IsA<gtk::Window>>(
    ui: &UiRefs,
    state: &Rc<RefCell<AppState>>,
    transient_for: &W,
) {
    let language = state.borrow().settings.language;
    let chooser = FileChooserNative::builder()
        .title(text_for(language, "choose_notes_folder"))
        .transient_for(transient_for)
        .action(FileChooserAction::SelectFolder)
        .accept_label(text_for(language, "select"))
        .cancel_label(text_for(language, "cancel"))
        .build();
    let state = state.clone();
    let ui = ui.clone();
    chooser.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept
            && let Some(file) = dialog.file()
            && let Some(path) = file.path()
        {
            let mut st = state.borrow_mut();
            st.settings.notes_dir = path.clone();
            save_settings(&st.settings);
            drop(st);
            ensure_notes_dir(&path);
            repopulate_notes_list(&ui, &state);
        }
        dialog.destroy();
    });
    chooser.show();
}

fn insert_slash_item_from_index(
    index: i32,
    editor_buffer: &TextBuffer,
    menu_box: &GtkBox,
    menu_state: &Rc<RefCell<SlashMenuState>>,
) -> bool {
    if index < 0 {
        return false;
    }

    let (item, slash_offset, replace_end_offset) = {
        let st = menu_state.borrow();
        let item = st.items.get(index as usize).cloned();
        (item, st.slash_offset, st.replace_end_offset)
    };

    let (Some(item), Some(slash_offset), Some(replace_end_offset)) =
        (item, slash_offset, replace_end_offset)
    else {
        hide_slash_menu(menu_box, menu_state);
        return false;
    };

    {
        let mut st = menu_state.borrow_mut();
        st.suppress_change = true;
    }
    let mut slash_start = editor_buffer.iter_at_offset(slash_offset);
    let mut slash_end = editor_buffer.iter_at_offset(replace_end_offset);
    editor_buffer.delete(&mut slash_start, &mut slash_end);
    match item.action {
        CommandMenuAction::NoteLink(title) => {
            editor_buffer.insert_at_cursor(&format!("[[{title}]]"))
        }
        CommandMenuAction::InsertText(text) => editor_buffer.insert_at_cursor(&text),
        _ => {}
    }
    {
        let mut st = menu_state.borrow_mut();
        st.suppress_change = false;
    }
    hide_slash_menu(menu_box, menu_state);
    true
}

fn execute_command_item_from_index(
    index: i32,
    ui: &UiRefs,
    app_state: &Rc<RefCell<AppState>>,
    editor: &TextView,
    command_input: &Entry,
    menu_box: &GtkBox,
    menu_state: &Rc<RefCell<CommandPaletteState>>,
) -> bool {
    if index < 0 {
        return false;
    }

    let item = {
        let st = menu_state.borrow();
        st.items.get(index as usize).cloned()
    };

    let Some(item) = item else {
        hide_command_palette(menu_box, menu_state);
        return false;
    };

    match item.action {
        CommandMenuAction::InsertText(text) => {
            ui.editor_buffer.insert_at_cursor(&text);
            app_state.borrow_mut().dirty = true;
        }
        CommandMenuAction::NoOp => return false,
        CommandMenuAction::NoteLink(title) => {
            ui.editor_buffer.insert_at_cursor(&format!("[[{title}]]"));
            app_state.borrow_mut().dirty = true;
        }
        CommandMenuAction::OpenNote(path) => {
            if app_state.borrow().dirty {
                save_current_note(ui, app_state);
            }
            load_note_into_editor(&path, ui, app_state);
        }
        CommandMenuAction::CreateNote => {
            if app_state.borrow().dirty {
                save_current_note(ui, app_state);
            }
            create_new_note(ui, app_state);
        }
        CommandMenuAction::ToggleSidebar => {
            let shown = ui.split_view.shows_sidebar();
            ui.split_view.set_show_sidebar(!shown);
        }
        CommandMenuAction::OpenSettings => {
            let settings_window = build_settings_window(ui, app_state);
            settings_window.present();
        }
        CommandMenuAction::SetLanguage(language) => {
            {
                let mut st = app_state.borrow_mut();
                st.settings.language = language;
                save_settings(&st.settings);
            }
            update_translations(ui, language);
        }
        CommandMenuAction::SetTheme(theme) => {
            {
                let mut st = app_state.borrow_mut();
                st.settings.theme = theme;
                save_settings(&st.settings);
            }
            apply_theme(theme);
        }
        CommandMenuAction::ChooseNotesFolder => {
            choose_notes_folder(ui, app_state, &ui.window);
        }
    }

    command_input.set_text("");
    hide_command_palette(menu_box, menu_state);
    editor.grab_focus();
    true
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
        let subtitle = note_subtitle(&path);

        let row = ListBoxRow::new();
        row.set_selectable(true);
        row.set_activatable(true);
        row.set_widget_name(path.to_string_lossy().as_ref());
        let note_row = ActionRow::builder()
            .title(&title)
            .subtitle(&subtitle)
            .activatable(true)
            .build();
        row.set_child(Some(&note_row));
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
        let group = group.clone();
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
            group.set_title(text_for(language, "settings_title"));
            row.set_title(text_for(language, "language"));
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
            let language = state.borrow().settings.language;
            let chooser = FileChooserNative::builder()
                .title(text_for(language, "choose_notes_folder"))
                .transient_for(&settings_window)
                .action(FileChooserAction::SelectFolder)
                .accept_label(text_for(language, "select"))
                .cancel_label(text_for(language, "cancel"))
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
    install_command_palette_css();

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
    let title = WindowTitle::new(text_for(initial_settings.language, "title"), "");
    header.set_title_widget(Some(&title));

    let toolbar_view = ToolbarView::new();
    let split_view = OverlaySplitView::new();
    split_view.set_show_sidebar(true);

    let sidebar = GtkBox::new(Orientation::Vertical, 0);
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
    notes_list.add_css_class("navigation-sidebar");
    let notes_scroller = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
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
    editor_scroller.set_child(Some(&editor));
    let editor_overlay = Overlay::new();
    editor_overlay.set_child(Some(&editor_scroller));

    let slash_menu_box = GtkBox::new(Orientation::Vertical, 0);
    slash_menu_box.set_halign(Align::Start);
    slash_menu_box.set_valign(Align::Start);
    slash_menu_box.set_margin_top(10);
    slash_menu_box.set_margin_start(8);
    slash_menu_box.set_margin_end(8);
    slash_menu_box.add_css_class("card");
    slash_menu_box.set_size_request(360, -1);
    slash_menu_box.set_visible(false);

    let slash_menu_list = ListBox::new();
    slash_menu_list.set_selection_mode(SelectionMode::Single);
    slash_menu_list.set_activate_on_single_click(true);
    let slash_menu_scroller = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(70)
        .max_content_height(260)
        .build();
    slash_menu_scroller.set_child(Some(&slash_menu_list));
    slash_menu_box.append(&slash_menu_scroller);
    editor_overlay.add_overlay(&slash_menu_box);

    let command_palette_box = GtkBox::new(Orientation::Vertical, 0);
    command_palette_box.set_halign(Align::Center);
    command_palette_box.set_valign(Align::Start);
    command_palette_box.set_margin_top(72);
    command_palette_box.set_margin_start(16);
    command_palette_box.set_margin_end(16);
    command_palette_box.add_css_class("command-palette");
    command_palette_box.set_size_request(980, -1);
    command_palette_box.set_visible(false);

    let command_palette_list = ListBox::new();
    command_palette_list.set_selection_mode(SelectionMode::Single);
    command_palette_list.set_activate_on_single_click(true);
    let command_palette_scroller = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(240)
        .max_content_height(520)
        .build();
    command_palette_scroller.set_child(Some(&command_palette_list));

    let command_input = Entry::new();
    command_input.set_placeholder_text(Some("Type a command or search notes"));
    command_input.set_margin_top(8);
    command_input.set_margin_start(8);
    command_input.set_margin_end(8);
    command_input.set_margin_bottom(4);
    command_input.set_visible(true);

    command_palette_box.append(&command_input);
    command_palette_box.append(&command_palette_scroller);
    editor_overlay.add_overlay(&command_palette_box);
    resize_command_palette(&window, &command_palette_box, &command_palette_scroller);

    split_view.set_sidebar(Some(&sidebar));
    split_view.set_content(Some(&editor_overlay));
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
    let slash_menu_state = Rc::new(RefCell::new(SlashMenuState::default()));
    let command_palette_state = Rc::new(RefCell::new(CommandPaletteState::default()));

    repopulate_notes_list(&ui, &state);

    {
        let ui = ui.clone();
        let state = state.clone();
        let slash_menu_box = slash_menu_box.clone();
        let slash_menu_state = slash_menu_state.clone();
        let command_palette_box = command_palette_box.clone();
        let command_palette_state = command_palette_state.clone();
        notes_list.connect_row_selected(move |_list, row| {
            if let Some(row) = row {
                if state.borrow().dirty {
                    save_current_note(&ui, &state);
                }
                hide_slash_menu(&slash_menu_box, &slash_menu_state);
                hide_command_palette(&command_palette_box, &command_palette_state);
                let path_text = row.widget_name().to_string();
                load_note_into_editor(Path::new(&path_text), &ui, &state);
            }
        });
    }

    {
        let ui = ui.clone();
        let state = state.clone();
        let slash_menu_box = slash_menu_box.clone();
        let slash_menu_state = slash_menu_state.clone();
        let command_palette_box = command_palette_box.clone();
        let command_palette_state = command_palette_state.clone();
        new_btn.connect_clicked(move |_| {
            if state.borrow().dirty {
                save_current_note(&ui, &state);
            }
            hide_slash_menu(&slash_menu_box, &slash_menu_state);
            hide_command_palette(&command_palette_box, &command_palette_state);
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
        let editor_buffer = editor_buffer.clone();
        let query_buffer = editor_buffer.clone();
        let editor = editor.clone();
        let slash_menu_box = slash_menu_box.clone();
        let slash_menu_list = slash_menu_list.clone();
        let slash_menu_state = slash_menu_state.clone();
        editor_buffer.connect_changed(move |_| {
            if slash_menu_state.borrow().suppress_change {
                return;
            }

            let mut st = state.borrow_mut();
            if st.loading_note {
                drop(st);
                hide_slash_menu(&slash_menu_box, &slash_menu_state);
                return;
            }
            st.dirty = true;
            drop(st);

            if let Some((slash_offset, replace_end_offset, query)) =
                slash_query_at_cursor(&query_buffer)
            {
                if query.chars().any(|ch| ch.is_whitespace()) {
                    hide_slash_menu(&slash_menu_box, &slash_menu_state);
                    return;
                }

                let items = slash_menu_items(&state, &query);
                if items.is_empty() {
                    hide_slash_menu(&slash_menu_box, &slash_menu_state);
                    return;
                }

                populate_command_list(&slash_menu_list, &items);
                position_command_menu(&editor, &slash_menu_box);
                slash_menu_box.set_visible(true);
                if let Some(first_row) = slash_menu_list.row_at_index(0) {
                    slash_menu_list.select_row(Some(&first_row));
                }

                let mut menu_state = slash_menu_state.borrow_mut();
                menu_state.visible = true;
                menu_state.slash_offset = Some(slash_offset);
                menu_state.replace_end_offset = Some(replace_end_offset);
                menu_state.items = items;
            } else if slash_menu_state.borrow().visible {
                hide_slash_menu(&slash_menu_box, &slash_menu_state);
            }
        });
    }

    {
        let state = state.clone();
        let editor_buffer = editor_buffer.clone();
        let slash_menu_box = slash_menu_box.clone();
        let slash_menu_state = slash_menu_state.clone();
        slash_menu_list.connect_row_activated(move |_list, row| {
            if insert_slash_item_from_index(
                row.index(),
                &editor_buffer,
                &slash_menu_box,
                &slash_menu_state,
            ) {
                state.borrow_mut().dirty = true;
            }
        });
    }

    {
        let editor_buffer = editor_buffer.clone();
        let slash_menu_list = slash_menu_list.clone();
        let slash_menu_box = slash_menu_box.clone();
        let slash_menu_state = slash_menu_state.clone();
        let state = state.clone();
        let key_controller = EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |_controller, key, _code, _state| {
            if !slash_menu_state.borrow().visible {
                return glib::Propagation::Proceed;
            }

            if key == gtk::gdk::Key::Up {
                let selected_index = slash_menu_list
                    .selected_row()
                    .map(|r| r.index())
                    .unwrap_or(0);
                let next_index = (selected_index - 1).max(0);
                if let Some(row) = slash_menu_list.row_at_index(next_index) {
                    slash_menu_list.select_row(Some(&row));
                    row.grab_focus();
                }
                return glib::Propagation::Stop;
            }

            if key == gtk::gdk::Key::Down {
                let selected_index = slash_menu_list
                    .selected_row()
                    .map(|r| r.index())
                    .unwrap_or(-1);
                let next_index = selected_index + 1;
                if let Some(row) = slash_menu_list.row_at_index(next_index) {
                    slash_menu_list.select_row(Some(&row));
                    row.grab_focus();
                }
                return glib::Propagation::Stop;
            }

            if key == gtk::gdk::Key::Return || key == gtk::gdk::Key::KP_Enter {
                let selected_index = slash_menu_list
                    .selected_row()
                    .map(|r| r.index())
                    .unwrap_or(0);
                if insert_slash_item_from_index(
                    selected_index,
                    &editor_buffer,
                    &slash_menu_box,
                    &slash_menu_state,
                ) {
                    state.borrow_mut().dirty = true;
                    return glib::Propagation::Stop;
                }
            }

            if key == gtk::gdk::Key::Escape {
                hide_slash_menu(&slash_menu_box, &slash_menu_state);
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });
        editor.add_controller(key_controller);
    }

    {
        let ui = ui.clone();
        let editor = editor.clone();
        let state = state.clone();
        let command_input = command_input.clone();
        let command_palette_box = command_palette_box.clone();
        let command_palette_list = command_palette_list.clone();
        let command_palette_state = command_palette_state.clone();
        command_palette_list.connect_row_activated(move |_list, row| {
            let _ = execute_command_item_from_index(
                row.index(),
                &ui,
                &state,
                &editor,
                &command_input,
                &command_palette_box,
                &command_palette_state,
            );
        });
    }

    {
        let ui = ui.clone();
        let editor = editor.clone();
        let state = state.clone();
        let command_input = command_input.clone();
        let command_palette_box = command_palette_box.clone();
        let command_palette_list = command_palette_list.clone();
        let command_palette_state = command_palette_state.clone();
        command_input.clone().connect_activate(move |_| {
            if !command_palette_state.borrow().visible {
                return;
            }
            let selected_index = command_palette_list
                .selected_row()
                .map(|r| r.index())
                .unwrap_or(0);
            let _ = execute_command_item_from_index(
                selected_index,
                &ui,
                &state,
                &editor,
                &command_input,
                &command_palette_box,
                &command_palette_state,
            );
        });
    }

    {
        let state = state.clone();
        let command_input = command_input.clone();
        let command_palette_box = command_palette_box.clone();
        let command_palette_list = command_palette_list.clone();
        let command_palette_state = command_palette_state.clone();
        command_input.clone().connect_changed(move |entry| {
            if !command_palette_state.borrow().visible {
                return;
            }
            let items = command_bar_items(&state, &entry.text());
            if items.is_empty() {
                hide_command_palette(&command_palette_box, &command_palette_state);
                return;
            }
            populate_command_list(&command_palette_list, &items);
            if let Some(first_row) = command_palette_list.row_at_index(0) {
                command_palette_list.select_row(Some(&first_row));
            }
            command_palette_box.set_visible(true);
            let mut st = command_palette_state.borrow_mut();
            st.visible = true;
            st.items = items;
        });
    }

    {
        let ui = ui.clone();
        let state = state.clone();
        let editor = editor.clone();
        let command_input = command_input.clone();
        let command_palette_box = command_palette_box.clone();
        let command_palette_list = command_palette_list.clone();
        let command_palette_state = command_palette_state.clone();
        let command_input_for_action = command_input.clone();
        let editor_for_action = editor.clone();
        let input_key_controller = EventControllerKey::new();
        input_key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        input_key_controller.connect_key_pressed(move |_controller, key, _code, _mods| {
            if !command_palette_state.borrow().visible {
                return glib::Propagation::Proceed;
            }
            if key == gtk::gdk::Key::Up {
                let selected_index = command_palette_list
                    .selected_row()
                    .map(|r| r.index())
                    .unwrap_or(0);
                let next_index = (selected_index - 1).max(0);
                if let Some(row) = command_palette_list.row_at_index(next_index) {
                    command_palette_list.select_row(Some(&row));
                    row.grab_focus();
                }
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::Down {
                let selected_index = command_palette_list
                    .selected_row()
                    .map(|r| r.index())
                    .unwrap_or(-1);
                let next_index = selected_index + 1;
                if let Some(row) = command_palette_list.row_at_index(next_index) {
                    command_palette_list.select_row(Some(&row));
                    row.grab_focus();
                }
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::Return || key == gtk::gdk::Key::KP_Enter {
                let selected_index = command_palette_list
                    .selected_row()
                    .map(|r| r.index())
                    .unwrap_or(0);
                if execute_command_item_from_index(
                    selected_index,
                    &ui,
                    &state,
                    &editor_for_action,
                    &command_input_for_action,
                    &command_palette_box,
                    &command_palette_state,
                ) {
                    return glib::Propagation::Stop;
                }
            }
            if key == gtk::gdk::Key::Escape {
                command_input_for_action.set_text("");
                hide_command_palette(&command_palette_box, &command_palette_state);
                editor_for_action.grab_focus();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        command_input.add_controller(input_key_controller);
    }

    {
        let state = state.clone();
        let command_input = command_input.clone();
        let command_palette_box = command_palette_box.clone();
        let command_palette_list = command_palette_list.clone();
        let command_palette_scroller = command_palette_scroller.clone();
        let command_palette_state = command_palette_state.clone();
        let slash_menu_box = slash_menu_box.clone();
        let slash_menu_state = slash_menu_state.clone();
        let window_for_resize = window.clone();
        let window_key_controller = EventControllerKey::new();
        window_key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        window_key_controller.connect_key_pressed(move |_controller, key, _code, mods| {
            if key == gtk::gdk::Key::k && mods.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                let is_open = command_palette_state.borrow().visible;
                if is_open {
                    command_input.set_text("");
                    hide_command_palette(&command_palette_box, &command_palette_state);
                    return glib::Propagation::Stop;
                }

                hide_slash_menu(&slash_menu_box, &slash_menu_state);
                let items = command_bar_items(&state, "");
                populate_command_list(&command_palette_list, &items);
                resize_command_palette(
                    &window_for_resize,
                    &command_palette_box,
                    &command_palette_scroller,
                );
                command_input.set_text("");
                command_palette_box.set_visible(true);
                if let Some(first_row) = command_palette_list.row_at_index(0) {
                    command_palette_list.select_row(Some(&first_row));
                }

                let mut st = command_palette_state.borrow_mut();
                st.visible = true;
                st.items = items;

                command_input.grab_focus();
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::Escape && command_palette_state.borrow().visible {
                command_input.set_text("");
                hide_command_palette(&command_palette_box, &command_palette_state);
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::Escape && slash_menu_state.borrow().visible {
                hide_slash_menu(&slash_menu_box, &slash_menu_state);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(window_key_controller);
    }

    {
        let command_palette_box = command_palette_box.clone();
        let command_palette_scroller = command_palette_scroller.clone();
        window.connect_notify_local(Some("default-width"), move |win, _| {
            resize_command_palette(win, &command_palette_box, &command_palette_scroller);
        });
    }

    {
        let command_palette_box = command_palette_box.clone();
        let command_palette_scroller = command_palette_scroller.clone();
        window.connect_notify_local(Some("default-height"), move |win, _| {
            resize_command_palette(win, &command_palette_box, &command_palette_scroller);
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

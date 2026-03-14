mod command_bar;
mod extensions;
mod l10n;
mod settings;
mod sidebar;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use adw::{
    Application, ApplicationWindow, ColorScheme, HeaderBar, OverlaySplitView, StyleManager,
    ToolbarView, WindowTitle,
};
use chrono::Local;
use gtk::glib;
use gtk::gio;
use gtk::{
    Align, Box as GtkBox, Button, Entry, EventControllerKey, EventControllerMotion, FileChooserAction,
    FileChooserNative, ListBox, Orientation, Overlay, PolicyType, ScrolledWindow, SelectionMode,
    TextBuffer, TextView,
};
use serde::{Deserialize, Serialize};

use command_bar::{
    command_bar_items, execute_command_item_from_index, hide_command_palette, hide_slash_menu,
    insert_slash_item_from_index, install_command_palette_css, populate_command_list,
    position_command_menu, resize_command_palette, slash_menu_items, slash_query_at_cursor,
    CommandPaletteState, SlashMenuState,
};
use extensions::ExtensionRegistry;
use l10n::{text_for, update_translations, Language};
use settings::build_settings_window;
use sidebar::repopulate_notes_list;

const APP_ID: &str = "local.nete.notes";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn from_selected(index: u32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn selected(self) -> u32 {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
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
pub struct UiRefs {
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
pub struct AppState {
    settings: AppSettings,
    current_note: Option<PathBuf>,
    dirty: bool,
    loading_note: bool,
    extension_registry: ExtensionRegistry,
    title_cache: HashMap<PathBuf, String>,
    last_saved_content: Option<String>,
    save_debounce_timer: Option<glib::SourceId>,
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Nete")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

pub fn get_cached_title(state: &Rc<RefCell<AppState>>, path: &Path, filename: &str) -> String {
    {
        let st = state.borrow();
        if let Some(cached) = st.title_cache.get(path) {
            return cached.clone();
        }
    }

    let title = read_file_for_title(path)
        .map(|txt| note_title_from_markdown(&txt, filename))
        .unwrap_or_else(|| filename.to_string());

    {
        let mut st = state.borrow_mut();
        st.title_cache.insert(path.to_path_buf(), title.clone());
    }

    title
}

pub fn invalidate_title_cache(state: &Rc<RefCell<AppState>>, path: Option<&Path>) {
    let mut st = state.borrow_mut();
    match path {
        Some(p) => {
            st.title_cache.remove(p);
        }
        None => {
            st.title_cache.clear();
        }
    }
}

pub fn clear_title_cache(state: &Rc<RefCell<AppState>>) {
    state.borrow_mut().title_cache.clear();
}

fn load_settings() -> AppSettings {
    let path = settings_path();
    let Ok(content) = fs::read_to_string(path) else {
        return AppSettings::default();
    };

    toml::from_str(&content).unwrap_or_else(|_| AppSettings::default())
}

pub fn apply_theme(theme: ThemeMode) {
    let style = StyleManager::default();
    match theme {
        ThemeMode::System => style.set_color_scheme(ColorScheme::Default),
        ThemeMode::Light => style.set_color_scheme(ColorScheme::ForceLight),
        ThemeMode::Dark => style.set_color_scheme(ColorScheme::ForceDark),
    };
}

pub fn note_title_from_markdown(content: &str, fallback: &str) -> String {
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

const READ_TITLE_BYTES: usize = 1024;

pub fn read_file_for_title(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = vec![0u8; READ_TITLE_BYTES];
    let bytes_read = std::io::Read::read(&mut reader, &mut buffer).ok()?;
    buffer.truncate(bytes_read);
    String::from_utf8(buffer).ok()
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

pub fn list_markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = vec![];
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }

    let mut files_with_time: Vec<(PathBuf, std::time::SystemTime)> = files
        .into_iter()
        .filter_map(|p| {
            fs::metadata(&p)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| (p, t))
        })
        .collect();

    files_with_time.sort_by(|a, b| b.1.cmp(&a.1));

    files_with_time.into_iter().map(|(p, _)| p).collect()
}

pub fn linkable_note_titles(state: &Rc<RefCell<AppState>>) -> Vec<String> {
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
        let title = get_cached_title(state, &path, &filename);
        titles.push(title);
    }

    titles
}

pub fn save_settings(settings: &AppSettings) {
    let cfg_dir = config_dir();
    if let Err(e) = fs::create_dir_all(&cfg_dir) {
        eprintln!("Failed to create config directory: {}", e);
        return;
    }
    let path = settings_path();
    let Ok(serialized) = toml::to_string_pretty(settings) else {
        eprintln!("Failed to serialize settings");
        return;
    };
    if let Err(e) = fs::write(path, serialized) {
        eprintln!("Failed to write settings: {}", e);
    }
}

fn ensure_notes_dir(path: &Path) {
    if let Err(e) = fs::create_dir_all(path) {
        eprintln!("Failed to create notes directory: {}", e);
    }
}

pub fn save_current_note(ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
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

        // Only save if content actually changed
        let last_saved = state.borrow().last_saved_content.clone();
        if last_saved.as_ref() == Some(&text) {
            return;
        }

        if let Err(e) = fs::write(&path, &text) {
            eprintln!("Failed to save note: {}", e);
            return;
        }
        state.borrow_mut().dirty = false;
        state.borrow_mut().last_saved_content = Some(text);
        invalidate_title_cache(state, Some(&path));
    }
}

pub fn mark_content_changed(ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
    let text = ui
        .editor_buffer
        .text(
            &ui.editor_buffer.start_iter(),
            &ui.editor_buffer.end_iter(),
            true,
        )
        .to_string();

    let has_changes = state.borrow().last_saved_content.as_ref() != Some(&text);
    if has_changes && !state.borrow().dirty {
        state.borrow_mut().dirty = true;
        schedule_debounced_save(ui, state);
    } else if has_changes {
        schedule_debounced_save(ui, state);
    }
}

fn schedule_debounced_save(ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
    let ui = ui.clone();
    let state_for_timer = state.clone();

    // Cancel existing timer first to avoid memory leaks
    let old_timer = state.borrow_mut().save_debounce_timer.take();
    if let Some(t) = old_timer {
        t.remove();
    }

    // Schedule save after 2 seconds of inactivity
    let timer = glib::timeout_add_seconds_local(2, move || {
        if state_for_timer.borrow().dirty {
            save_current_note(&ui, &state_for_timer);
            crate::repopulate_notes_list(&ui, &state_for_timer);
        }
        state_for_timer.borrow_mut().save_debounce_timer = None;
        glib::ControlFlow::Continue
    });

    // Store the timer in state
    state.borrow_mut().save_debounce_timer = Some(timer);
}

pub fn choose_notes_folder<W: IsA<gtk::Window>>(
    ui: &UiRefs,
    state: &Rc<RefCell<AppState>>,
    transient_for: &W,
) {
    use l10n::text_for;
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
        if response == gtk::ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    let mut st = state.borrow_mut();
                    st.settings.notes_dir = path.clone();
                    save_settings(&st.settings);
                    drop(st);
                    ensure_notes_dir(&path);
                    repopulate_notes_list(&ui, &state);
                }
            }
        }
        dialog.destroy();
    });
    chooser.show();
}

pub fn load_note_into_editor(path: &Path, ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load note {:?}: {}", path, e);
            String::new()
        }
    };
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
        st.last_saved_content = Some(content);
    }
    invalidate_title_cache(state, Some(path));
}

/// Find a note by title (case-insensitive match)
pub fn find_note_by_title(notes_dir: &Path, title: &str) -> Option<PathBuf> {
    for path in list_markdown_files(notes_dir) {
        let content = fs::read_to_string(&path).unwrap_or_default();
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("note.md")
            .to_string();
        let note_title = note_title_from_markdown(&content, &filename);
        if note_title.to_lowercase() == title.to_lowercase() {
            return Some(path);
        }
    }
    None
}

/// Extract all wiki-style links [[Title]] from text
pub fn extract_wiki_links(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' && i + 1 < chars.len() && chars[i + 1] == '[' {
            i += 2;
            let mut link = String::new();
            let mut found_close = false;
            while i < chars.len() {
                if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == ']' {
                    i += 2;
                    found_close = true;
                    break;
                }
                link.push(chars[i]);
                i += 1;
            }
            if found_close && !link.is_empty() {
                links.push(link);
            }
            if !found_close {
                break;
            }
        } else {
            i += 1;
        }
    }
    links
}

/// Handle link click - opens wiki link or external URL
pub fn handle_link_click(
    ui: &UiRefs,
    state: &Rc<RefCell<AppState>>,
    link_text: &str,
) {
    let notes_dir = state.borrow().settings.notes_dir.clone();

    // Check if it's a wiki link [[Note Title]]
    if link_text.starts_with('[') {
        // External link [text](url)
        if let Some(url_start) = link_text.find("](") {
            let url = &link_text[url_start + 2..link_text.len() - 1];
            // Use gio to open URL via application
            let launcher = gio::AppLaunchContext::new();
            let _ = gio::AppInfo::launch_default_for_uri(url, Some(&launcher));
            return;
        }

        // Wiki link [[Title]]
        let title = &link_text[2..link_text.len() - 2];
        if let Some(path) = find_note_by_title(&notes_dir, title) {
            if state.borrow().dirty {
                save_current_note(ui, state);
            }
            load_note_into_editor(&path, ui, state);
        }
    } else if link_text.starts_with("http://") || link_text.starts_with("https://") {
        // Plain URL
        let launcher = gio::AppLaunchContext::new();
        let _ = gio::AppInfo::launch_default_for_uri(link_text, Some(&launcher));
    } else {
        // Try as wiki link
        if let Some(path) = find_note_by_title(&notes_dir, link_text) {
            if state.borrow().dirty {
                save_current_note(ui, state);
            }
            load_note_into_editor(&path, ui, state);
        }
    }
}

fn setup_link_click_handler(
    ui: &UiRefs,
    state: &Rc<RefCell<AppState>>,
    editor: &TextView,
) {
    let editor_for_motion = editor.clone();
    let editor_for_click = editor.clone();
    let state = state.clone();
    let ui = ui.clone();

    let motion_controller = EventControllerMotion::new();
    motion_controller.connect_enter(move |_controller, _x, _y| {
        let buffer = editor_for_motion.buffer();
        let cursor = buffer.iter_at_mark(&buffer.get_insert());
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let full_text = buffer.text(&start, &end, true).to_string();

        // Check if cursor is over a link
        let cursor_offset = cursor.offset() as usize;
        let link = find_link_at_position(&full_text, cursor_offset);
        if link.is_some() {
            editor_for_motion.set_cursor(gtk::gdk::Cursor::from_name("pointer", None).as_ref());
        } else {
            editor_for_motion.set_cursor(gtk::gdk::Cursor::from_name("text", None).as_ref());
        }
    });
    editor.add_controller(motion_controller);

    let click_controller = gtk::GestureClick::new();
    click_controller.connect_pressed(move |_controller, _n, x, y| {
        let x_i32 = x as i32;
        let y_i32 = y as i32;
        if let Some(iter) = editor_for_click.iter_at_location(x_i32, y_i32) {
            let buffer = editor_for_click.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let full_text = buffer.text(&start, &end, true).to_string();

            let cursor_offset = iter.offset() as usize;
            if let Some(link) = find_link_at_position(&full_text, cursor_offset) {
                handle_link_click(&ui, &state, &link);
            }
        }
    });
    editor.add_controller(click_controller);
}

fn find_link_at_position(text: &str, position: usize) -> Option<String> {
    // Find wiki links [[...]]
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' && i + 1 < chars.len() && chars[i + 1] == '[' {
            let link_start = i;
            i += 2;
            let mut link_content = String::new();
            let mut found_close = false;
            while i < chars.len() {
                if chars[i] == ']' && i + 1 < chars.len() && chars[i + 1] == ']' {
                    let link_end = i + 2;
                    if link_start <= position && position <= link_end {
                        return Some(format!("[[{}]]", link_content));
                    }
                    i = link_end;
                    found_close = true;
                    break;
                }
                link_content.push(chars[i]);
                i += 1;
            }
            if !found_close {
                break;
            }
        } else if chars[i] == '(' && i > 0 && chars[i - 1] == ']' {
            // Might be a markdown link [text](url)
            let url_start = i + 1;
            let mut j = i + 1;
            while j < chars.len() && chars[j] != ')' {
                j += 1;
            }
            if j > url_start && url_start <= position && position <= j + 1 {
                let url = text[url_start..j].to_string();
                return Some(format!("text]({})", url));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    None
}

pub fn create_new_note(ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
    let dir = state.borrow().settings.notes_dir.clone();
    ensure_notes_dir(&dir);
    let name = format!("note-{}.md", Local::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(name);
    let initial = "# New note\n";
    if let Err(e) = fs::write(&path, initial) {
        eprintln!("Failed to create new note: {}", e);
        return;
    }
    repopulate_notes_list(ui, state);
    load_note_into_editor(&path, ui, state);
}

fn build_ui(app: &Application) {
    install_command_palette_css();

    let initial_settings = load_settings();
    ensure_notes_dir(&initial_settings.notes_dir);
    save_settings(&initial_settings);
    apply_theme(initial_settings.theme);

    // Load and apply extensions
    let extension_registry = ExtensionRegistry::load_all();
    extension_registry.apply_themes();

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
    let root_overlay = Overlay::new();

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

    let state = Rc::new(RefCell::new(AppState {
        settings: initial_settings.clone(),
        extension_registry,
        ..Default::default()
    }));

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

    // Setup clickable links after ui and state are created
    setup_link_click_handler(&ui, &state, &editor);

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

    split_view.set_sidebar(Some(&sidebar));
    split_view.set_content(Some(&editor_overlay));
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&split_view));
    root_overlay.set_child(Some(&toolbar_view));
    command_palette_box.append(&command_input);
    command_palette_box.append(&command_palette_scroller);
    root_overlay.add_overlay(&command_palette_box);
    root_overlay.set_clip_overlay(&command_palette_box, false);
    resize_command_palette(&window, &command_palette_box, &command_palette_scroller);
    window.set_content(Some(&root_overlay));

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
                let path = Path::new(&path_text);
                if path.is_file() {
                    load_note_into_editor(path, &ui, &state);
                }
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
        let ui = ui.clone();
        editor_buffer.connect_changed(move |_| {
            if slash_menu_state.borrow().suppress_change {
                return;
            }

            let st = state.borrow_mut();
            if st.loading_note {
                drop(st);
                hide_slash_menu(&slash_menu_box, &slash_menu_state);
                return;
            }
            drop(st);
            mark_content_changed(&ui, &state);

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
        let state_for_slash = state.clone();
        slash_menu_list.connect_row_activated(move |_list, row| {
            if insert_slash_item_from_index(
                row.index(),
                &editor_buffer,
                &slash_menu_box,
                &slash_menu_state,
                &state_for_slash,
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
                let state_for_slash = state.clone();
                if insert_slash_item_from_index(
                    selected_index,
                    &editor_buffer,
                    &slash_menu_box,
                    &slash_menu_state,
                    &state_for_slash,
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

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Entry, ListBox, ListBoxRow, Orientation, ScrolledWindow, TextBuffer,
    TextView, TextWindowType,
};

use crate::extensions::{ExtensionContext, ExtensionResult};
use crate::l10n::{update_translations, Language};

#[derive(Default)]
pub struct SlashMenuState {
    pub items: Vec<CommandMenuItem>,
    pub slash_offset: Option<i32>,
    pub replace_end_offset: Option<i32>,
    pub visible: bool,
    pub suppress_change: bool,
}

#[derive(Default)]
pub struct CommandPaletteState {
    pub items: Vec<CommandMenuItem>,
    pub visible: bool,
}

#[derive(Clone)]
pub enum CommandMenuAction {
    NoteLink(String),
    InsertText(String),
    NoOp,
    OpenNote(PathBuf),
    CreateNote,
    ToggleSidebar,
    OpenSettings,
    SetLanguage(Language),
    SetTheme(crate::ThemeMode),
    ChooseNotesFolder,
    ExtensionCommand(crate::extensions::CommandDefinition),
    ExtensionSlashCommand(String, crate::extensions::SlashCommandDefinition),
}

impl CommandMenuAction {
    pub fn icon_name(&self) -> String {
        match self {
            Self::NoteLink(_) => "text-x-generic-symbolic".to_string(),
            Self::InsertText(_) => "applications-engineering-symbolic".to_string(),
            Self::NoOp => "dialog-warning-symbolic".to_string(),
            Self::OpenNote(_) => "text-x-generic-symbolic".to_string(),
            Self::CreateNote => "document-new-symbolic".to_string(),
            Self::ToggleSidebar => "sidebar-show-symbolic".to_string(),
            Self::OpenSettings | Self::ChooseNotesFolder => "emblem-system-symbolic".to_string(),
            Self::SetLanguage(_) => "preferences-desktop-locale-symbolic".to_string(),
            Self::SetTheme(_) => "weather-clear-night-symbolic".to_string(),
            Self::ExtensionCommand(def) => def
                .icon
                .clone()
                .unwrap_or_else(|| "application-x-addon-symbolic".to_string()),
            Self::ExtensionSlashCommand(_, _) => "application-x-addon-symbolic".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct CommandMenuItem {
    pub label: String,
    pub action: CommandMenuAction,
}

pub fn install_command_palette_css() {
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

pub fn slash_menu_items(state: &Rc<RefCell<crate::AppState>>, query: &str) -> Vec<CommandMenuItem> {
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

    let note_items = crate::linkable_note_titles(state)
        .into_iter()
        .filter(|title| title.to_lowercase().contains(&normalized_query))
        .map(|title| CommandMenuItem {
            label: title.clone(),
            action: CommandMenuAction::NoteLink(title),
        });
    items.extend(note_items);

    let ext_items: Vec<CommandMenuItem> = state
        .borrow()
        .extension_registry
        .get_extension_slash_commands()
        .into_iter()
        .filter(|(cmd, _)| {
            let matches_label = cmd.label.to_lowercase().contains(&normalized_query);
            let matches_alias = cmd
                .aliases
                .iter()
                .any(|a| a.to_lowercase().contains(&normalized_query));
            matches_label || matches_alias
        })
        .map(|(cmd, ext_id)| CommandMenuItem {
            label: cmd.label.clone(),
            action: CommandMenuAction::ExtensionSlashCommand(ext_id, cmd),
        })
        .collect();
    items.extend(ext_items);

    items
}

pub fn command_bar_items(
    state: &Rc<RefCell<crate::AppState>>,
    query: &str,
) -> Vec<CommandMenuItem> {
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
            action: CommandMenuAction::SetTheme(crate::ThemeMode::System),
        },
        CommandMenuItem {
            label: "Theme: Light".to_string(),
            action: CommandMenuAction::SetTheme(crate::ThemeMode::Light),
        },
        CommandMenuItem {
            label: "Theme: Dark".to_string(),
            action: CommandMenuAction::SetTheme(crate::ThemeMode::Dark),
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

    const SEARCH_READ_BYTES: usize = 8192;
    let notes_dir = state.borrow().settings.notes_dir.clone();
    let mut note_items = Vec::new();
    for path in crate::list_markdown_files(&notes_dir) {
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("note.md")
            .to_string();
        let content = std::fs::File::open(&path)
            .ok()
            .map(|file| {
                let mut reader = std::io::BufReader::new(file);
                let mut buffer = vec![0u8; SEARCH_READ_BYTES];
                let bytes_read = std::io::Read::read(&mut reader, &mut buffer).unwrap_or(0);
                buffer.truncate(bytes_read);
                String::from_utf8(buffer).unwrap_or_default()
            })
            .unwrap_or_default();
        let title = crate::note_title_from_markdown(&content, &filename);
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

    let ext_items: Vec<CommandMenuItem> = state
        .borrow()
        .extension_registry
        .get_extension_commands()
        .into_iter()
        .filter(|(cmd, _)| query_is_empty || cmd.label.to_lowercase().contains(&normalized_query))
        .map(|(cmd, _ext_id)| CommandMenuItem {
            label: cmd.label.clone(),
            action: CommandMenuAction::ExtensionCommand(cmd),
        })
        .collect();
    items.extend(ext_items);

    if items.is_empty() {
        items.push(CommandMenuItem {
            label: format!("No results for \"{}\"", query.trim()),
            action: CommandMenuAction::NoOp,
        });
    }
    items
}

pub fn slash_query_at_cursor(buffer: &TextBuffer) -> Option<(i32, i32, String)> {
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

pub fn position_command_menu(text_view: &TextView, menu_box: &GtkBox) {
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

pub fn populate_command_list(list: &ListBox, items: &[CommandMenuItem]) {
    use gtk::{Image, Label};
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
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

        let icon = Image::from_icon_name(&item_data.action.icon_name());
        icon.add_css_class("dim-label");
        icon.set_halign(Align::End);
        content.append(&icon);

        row.set_child(Some(&content));
        list.append(&row);
    }
}

pub fn hide_slash_menu(menu_box: &GtkBox, menu_state: &Rc<RefCell<SlashMenuState>>) {
    menu_box.set_visible(false);
    let mut st = menu_state.borrow_mut();
    st.visible = false;
    st.slash_offset = None;
    st.replace_end_offset = None;
    st.items.clear();
}

pub fn hide_command_palette(menu_box: &GtkBox, menu_state: &Rc<RefCell<CommandPaletteState>>) {
    menu_box.set_visible(false);
    let mut st = menu_state.borrow_mut();
    st.visible = false;
    st.items.clear();
}

pub fn resize_command_palette(
    window: &adw::ApplicationWindow,
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

pub fn insert_slash_item_from_index(
    index: i32,
    editor_buffer: &TextBuffer,
    menu_box: &GtkBox,
    menu_state: &Rc<RefCell<SlashMenuState>>,
    state: &Rc<RefCell<crate::AppState>>,
) -> bool {
    use crate::extensions::execute_extension_action;
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
    match &item.action {
        CommandMenuAction::NoteLink(title) => {
            editor_buffer.insert_at_cursor(&format!("[[{title}]]"))
        }
        CommandMenuAction::InsertText(text) => editor_buffer.insert_at_cursor(text),
        CommandMenuAction::ExtensionSlashCommand(_ext_id, cmd) => {
            let context = ExtensionContext {
                editor_text: Some(
                    editor_buffer
                        .text(&editor_buffer.start_iter(), &editor_buffer.end_iter(), true)
                        .to_string(),
                ),
                current_note_path: state.borrow().current_note.clone(),
                notes_dir: state.borrow().settings.notes_dir.clone(),
            };
            match execute_extension_action(&cmd.action, &cmd.text, &context) {
                ExtensionResult::InsertText(text) => {
                    editor_buffer.insert_at_cursor(&text);
                }
                _ => {}
            }
        }
        _ => {}
    }
    {
        let mut st = menu_state.borrow_mut();
        st.suppress_change = false;
    }
    hide_slash_menu(menu_box, menu_state);
    true
}

pub fn execute_command_item_from_index(
    index: i32,
    ui: &crate::UiRefs,
    app_state: &Rc<RefCell<crate::AppState>>,
    editor: &TextView,
    command_input: &Entry,
    menu_box: &GtkBox,
    menu_state: &Rc<RefCell<CommandPaletteState>>,
) -> bool {
    use crate::extensions::execute_extension_action;
    use crate::note_title_from_markdown;
    use crate::settings::build_settings_window;
    use crate::{choose_notes_folder, create_new_note, load_note_into_editor, save_current_note};
    use gtk::MessageDialog;
    use std::fs;

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

    match &item.action {
        CommandMenuAction::InsertText(text) => {
            ui.editor_buffer.insert_at_cursor(text);
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
            load_note_into_editor(path, ui, app_state);
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
                st.settings.language = *language;
                crate::save_settings(&st.settings);
            }
            update_translations(ui, *language);
        }
        CommandMenuAction::SetTheme(theme) => {
            {
                let mut st = app_state.borrow_mut();
                st.settings.theme = *theme;
                crate::save_settings(&st.settings);
            }
            crate::apply_theme(*theme);
            app_state.borrow().extension_registry.apply_themes();
        }
        CommandMenuAction::ChooseNotesFolder => {
            choose_notes_folder(ui, app_state, &ui.window);
        }
        CommandMenuAction::ExtensionCommand(cmd) => {
            let context = ExtensionContext {
                editor_text: Some(
                    ui.editor_buffer
                        .text(
                            &ui.editor_buffer.start_iter(),
                            &ui.editor_buffer.end_iter(),
                            true,
                        )
                        .to_string(),
                ),
                current_note_path: app_state.borrow().current_note.clone(),
                notes_dir: app_state.borrow().settings.notes_dir.clone(),
            };
            match execute_extension_action(&cmd.action, &cmd.text, &context) {
                ExtensionResult::InsertText(text) => {
                    ui.editor_buffer.insert_at_cursor(&text);
                    app_state.borrow_mut().dirty = true;
                }
                ExtensionResult::OpenNote(title) => {
                    let notes_dir = app_state.borrow().settings.notes_dir.clone();
                    for path in crate::list_markdown_files(&notes_dir) {
                        let content = fs::read_to_string(&path).unwrap_or_default();
                        let filename = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("note.md")
                            .to_string();
                        let note_title = note_title_from_markdown(&content, &filename);
                        if note_title == title {
                            if app_state.borrow().dirty {
                                save_current_note(ui, app_state);
                            }
                            load_note_into_editor(&path, ui, app_state);
                            break;
                        }
                    }
                }
                ExtensionResult::ShowMessage(msg) => {
                    let dialog = MessageDialog::new(
                        Some(&ui.window),
                        gtk::DialogFlags::MODAL,
                        gtk::MessageType::Info,
                        gtk::ButtonsType::Ok,
                        &msg,
                    );
                    dialog.connect_response(|dialog, _| {
                        dialog.close();
                    });
                    dialog.show();
                }
                ExtensionResult::NoOp => {}
            }
        }
        CommandMenuAction::ExtensionSlashCommand(_, _) => {}
    }

    command_input.set_text("");
    hide_command_palette(menu_box, menu_state);
    editor.grab_focus();
    true
}

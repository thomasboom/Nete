use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

use adw::prelude::*;
use adw::ActionRow;
use gtk::{ListBox, ListBoxRow};

use crate::note_subtitle;
use crate::note_title_from_markdown;
use crate::AppState;
use crate::UiRefs;

pub fn repopulate_notes_list(ui: &UiRefs, state: &Rc<RefCell<AppState>>) {
    clear_listbox(&ui.notes_list);

    let note_paths = {
        let st = state.borrow();
        crate::list_markdown_files(&st.settings.notes_dir)
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

fn clear_listbox(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

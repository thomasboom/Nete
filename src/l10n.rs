use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Language {
    English,
    Dutch,
}

impl Language {
    pub fn from_selected(index: u32) -> Self {
        match index {
            1 => Self::Dutch,
            _ => Self::English,
        }
    }

    pub fn selected(self) -> u32 {
        match self {
            Self::English => 0,
            Self::Dutch => 1,
        }
    }
}

pub fn text_for(lang: Language, key: &str) -> &'static str {
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

pub fn update_translations(ui: &crate::UiRefs, language: Language) {
    use gtk::prelude::GtkWindowExt;
    use gtk::prelude::WidgetExt;
    ui.window.set_title(Some(text_for(language, "title")));
    ui.header_title.set_title(text_for(language, "title"));
    ui.new_btn
        .set_tooltip_text(Some(text_for(language, "new_note")));
    ui.sidebar_btn
        .set_tooltip_text(Some(text_for(language, "toggle_sidebar")));
    ui.settings_btn
        .set_tooltip_text(Some(text_for(language, "settings")));
}

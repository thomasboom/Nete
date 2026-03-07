use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage, PreferencesWindow};
use gtk::{Button, StringList};

use crate::l10n::{text_for, update_translations, Language};
use crate::{apply_theme, save_settings, AppState, ThemeMode, UiRefs};

pub fn build_settings_window(ui: &UiRefs, state: &Rc<RefCell<AppState>>) -> PreferencesWindow {
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
            let chooser = gtk::FileChooserNative::builder()
                .title(text_for(language, "choose_notes_folder"))
                .transient_for(&settings_window)
                .action(gtk::FileChooserAction::SelectFolder)
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
                    crate::ensure_notes_dir(&path);
                    path_row.set_subtitle(path.to_string_lossy().as_ref());
                    crate::repopulate_notes_list(&ui, &state);
                }
                dialog.destroy();
            });
            chooser.show();
        });
    }

    settings_window
}

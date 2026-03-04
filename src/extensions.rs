//! App Extensions System
//!
//! This module provides a plugin system that allows developers to extend Nete Notes with:
//! - Custom themes via CSS
//! - Custom commands for the command bar
//! - Custom slash menu items
//!
//! Extensions are loaded from ~/.config/Nete/extensions/ or the platform equivalent.
//! Each extension has its own directory containing an extension.toml manifest.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Root directory for all extensions
pub fn extensions_dir() -> PathBuf {
    crate::config_dir().join("extensions")
}

/// Represents a loaded extension
#[derive(Clone, Debug)]
pub struct Extension {
    pub manifest: ExtensionManifest,
    pub path: PathBuf,
    pub enabled: bool,
}

/// Extension manifest parsed from extension.toml
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionManifest {
    #[serde(rename = "extension")]
    pub metadata: ExtensionMetadata,
    #[serde(default)]
    pub theme: Option<ThemeConfig>,
    #[serde(default)]
    pub commands: Vec<CommandDefinition>,
    #[serde(rename = "slash_commands", default)]
    pub slash_commands: Vec<SlashCommandDefinition>,
}

/// Metadata about the extension
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
}

/// Theme configuration for extensions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Path to CSS file relative to extension directory
    pub css_file: String,
}

/// A command definition for the command bar
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub action: ActionType,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub shortcut: Option<String>,
}

/// A slash command definition for the editor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlashCommandDefinition {
    pub id: String,
    pub label: String,
    pub action: ActionType,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// Types of actions an extension command can perform
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Insert text at cursor
    InsertText,
    /// Insert a note link [[Title]]
    InsertNoteLink,
    /// Open a specific note by title
    OpenNote,
    /// Execute an external command
    ExternalCommand,
    /// Toggle a boolean setting
    ToggleSetting,
    /// Set a setting value
    SetSetting,
}

/// The extension registry manages all loaded extensions
pub struct ExtensionRegistry {
    pub extensions: Vec<Extension>,
    pub enabled_extensions: Vec<String>,
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self {
            extensions: Vec::new(),
            enabled_extensions: Vec::new(),
        }
    }
}

impl ExtensionRegistry {
    /// Discover and load all extensions from the extensions directory
    pub fn load_all() -> Self {
        let mut registry = Self::default();
        let ext_dir = extensions_dir();

        if let Ok(entries) = fs::read_dir(&ext_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(ext) = Self::load_extension(&path) {
                        registry.extensions.push(ext);
                    }
                }
            }
        }

        // Load enabled extensions list
        registry.enabled_extensions = registry.load_enabled_list();

        // Mark extensions as enabled/disabled
        for ext in &mut registry.extensions {
            ext.enabled = registry
                .enabled_extensions
                .contains(&ext.manifest.metadata.id);
        }

        registry
    }

    /// Load a single extension from its directory
    fn load_extension(path: &Path) -> Option<Extension> {
        let manifest_path = path.join("extension.toml");
        let content = fs::read_to_string(&manifest_path).ok()?;
        let manifest: ExtensionManifest = toml::from_str(&content).ok()?;

        Some(Extension {
            manifest,
            path: path.to_path_buf(),
            enabled: false,
        })
    }

    /// Load the list of enabled extension IDs
    fn load_enabled_list(&self) -> Vec<String> {
        let enabled_path = extensions_dir().join("enabled.toml");
        if let Ok(content) = fs::read_to_string(&enabled_path) {
            if let Ok(list) = toml::from_str::<EnabledList>(&content) {
                return list.extensions;
            }
        }
        // Default: all extensions enabled
        self.extensions
            .iter()
            .map(|e| e.manifest.metadata.id.clone())
            .collect()
    }

    /// Save the list of enabled extensions
    pub fn save_enabled_list(&self) {
        let enabled_path = extensions_dir().join("enabled.toml");
        let list = EnabledList {
            extensions: self.enabled_extensions.clone(),
        };
        if let Ok(serialized) = toml::to_string_pretty(&list) {
            let _ = fs::write(&enabled_path, serialized);
        }
    }

    /// Enable an extension
    pub fn enable_extension(&mut self, id: &str) {
        if !self.enabled_extensions.contains(&id.to_string()) {
            self.enabled_extensions.push(id.to_string());
        }
        for ext in &mut self.extensions {
            if ext.manifest.metadata.id == id {
                ext.enabled = true;
            }
        }
        self.save_enabled_list();
    }

    /// Disable an extension
    pub fn disable_extension(&mut self, id: &str) {
        self.enabled_extensions.retain(|e| e != id);
        for ext in &mut self.extensions {
            if ext.manifest.metadata.id == id {
                ext.enabled = false;
            }
        }
        self.save_enabled_list();
    }

    /// Get all enabled extensions
    pub fn enabled(&self) -> impl Iterator<Item = &Extension> {
        self.extensions.iter().filter(|e| e.enabled)
    }

    /// Get all commands from enabled extensions
    pub fn get_extension_commands(&self) -> Vec<(CommandDefinition, String)> {
        let mut commands = Vec::new();
        for ext in self.enabled() {
            for cmd in &ext.manifest.commands {
                commands.push((cmd.clone(), ext.manifest.metadata.id.clone()));
            }
        }
        commands
    }

    /// Get all slash commands from enabled extensions
    pub fn get_extension_slash_commands(&self) -> Vec<(SlashCommandDefinition, String)> {
        let mut commands = Vec::new();
        for ext in self.enabled() {
            for cmd in &ext.manifest.slash_commands {
                commands.push((cmd.clone(), ext.manifest.metadata.id.clone()));
            }
        }
        commands
    }

    /// Get CSS content from all enabled theme extensions
    pub fn get_theme_css(&self) -> String {
        let mut css = String::new();
        for ext in self.enabled() {
            if let Some(theme) = &ext.manifest.theme {
                let css_path = ext.path.join(&theme.css_file);
                if let Ok(content) = fs::read_to_string(&css_path) {
                    css.push_str(&format!("/* Theme: {} */\n", ext.manifest.metadata.name));
                    css.push_str(&content);
                    css.push('\n');
                }
            }
        }
        css
    }

    /// Apply all extension themes to the application
    pub fn apply_themes(&self) {
        use gtk::gdk;

        let css = self.get_theme_css();
        if css.is_empty() {
            return;
        }

        let provider = gtk::CssProvider::new();
        provider.load_from_data(&css);

        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 10, // Higher priority than default
            );
        }
    }
}

#[derive(Serialize, Deserialize)]
struct EnabledList {
    extensions: Vec<String>,
}

/// Context passed to extension commands when executed
#[derive(Clone, Debug)]
pub struct ExtensionContext {
    pub editor_text: Option<String>,
    pub current_note_path: Option<PathBuf>,
    pub notes_dir: PathBuf,
}

/// Result of executing an extension command
#[derive(Clone, Debug)]
pub enum ExtensionResult {
    InsertText(String),
    OpenNote(String),
    ShowMessage(String),
    NoOp,
}

/// Helper to execute an extension action
pub fn execute_extension_action(
    action: &ActionType,
    text: &Option<String>,
    _context: &ExtensionContext,
) -> ExtensionResult {
    match action {
        ActionType::InsertText => {
            if let Some(text) = text {
                ExtensionResult::InsertText(text.clone())
            } else {
                ExtensionResult::NoOp
            }
        }
        ActionType::InsertNoteLink => {
            if let Some(title) = text {
                ExtensionResult::InsertText(format!("[[{}]]", title))
            } else {
                ExtensionResult::NoOp
            }
        }
        ActionType::OpenNote => {
            if let Some(title) = text {
                ExtensionResult::OpenNote(title.clone())
            } else {
                ExtensionResult::NoOp
            }
        }
        ActionType::ExternalCommand => {
            // For security, external commands would need explicit user permission
            // This is a placeholder for future implementation
            ExtensionResult::ShowMessage("External commands not yet implemented".to_string())
        }
        ActionType::ToggleSetting | ActionType::SetSetting => ExtensionResult::NoOp,
    }
}

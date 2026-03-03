use std::fs;
use std::path::Path;

use crate::error::AppResult;
use crate::models::PluginManifest;

#[derive(Debug, Clone)]
pub enum PluginEvent {
    AppStarted,
    NoteOpened(i64),
    NoteSaved(i64),
    SearchPerformed(String),
}

#[derive(Debug, Clone)]
pub struct PluginContext {
    pub app_name: String,
    pub local_data_authoritative: bool,
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub sandbox_root: String,
}

pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
}

impl PluginManager {
    pub fn discover(plugin_dir: &Path) -> AppResult<Self> {
        fs::create_dir_all(plugin_dir)?;

        let mut plugins = Vec::new();
        for entry in fs::read_dir(plugin_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("plugin.json");
            if !manifest_path.exists() {
                continue;
            }

            let raw = fs::read_to_string(&manifest_path)?;
            let manifest: PluginManifest = serde_json::from_str(&raw)?;
            plugins.push(LoadedPlugin {
                sandbox_root: path.to_string_lossy().to_string(),
                manifest,
                enabled: true,
            });
        }

        Ok(Self { plugins })
    }

    pub fn plugins(&self) -> &[LoadedPlugin] {
        &self.plugins
    }

    pub fn dispatch(&mut self, ctx: &PluginContext, event: PluginEvent) {
        for plugin in &mut self.plugins {
            if !plugin.enabled {
                continue;
            }

            let _sandbox_boundary = (&plugin.sandbox_root, &ctx.app_name);
            match &event {
                PluginEvent::AppStarted => {}
                PluginEvent::NoteOpened(_id) => {}
                PluginEvent::NoteSaved(_id) => {}
                PluginEvent::SearchPerformed(_query) => {}
            }
        }
    }
}


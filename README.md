# Nete

Nete is a local-first, privacy-first desktop Markdown notebook built in Rust. It provides a fast editor with live preview, backlinks, tags, graph navigation, and offline-first data ownership with optional cloud sync scaffolded but disabled by default.

## MVP Features

- Modern desktop UI with dark mode enabled by default
- SQLite-backed local datastore (WAL mode)
- Markdown editor with live preview
- Autosave + keyboard shortcuts
  - `Ctrl+N` create note
  - `Ctrl+S` save note
- Wiki-style backlinks via `[[Note Title]]`
- Tagging (`tags, comma-separated`)
- Search across title/content/tags
  - FTS5 full-text search + fuzzy title fallback
- Interactive graph panel for note relationships
- Extensible plugin discovery/runtime loading from local plugin directory
- Optional cloud sync state (disabled by default, local data authoritative)

## Build

Prerequisites:

- Rust toolchain (stable)
- Linux/macOS/Windows desktop environment for `eframe`

Build and verify:

```bash
cargo check
```

## Run

```bash
cargo run
```

On first launch, Nete creates a local config and data directory using platform-appropriate app paths, then initializes the SQLite database and a welcome note.

## Architecture

Project layout:

- `src/main.rs`: app entry + native window bootstrapping
- `src/app.rs`: top-level app runtime/state and UI composition
- `src/ui/theme.rs`: dark theme styling + visual defaults
- `src/config.rs`: app config loading/saving + default local paths
- `src/db.rs`: schema migrations + data access + indexing
- `src/models.rs`: domain data models
- `src/services.rs`: editor/search/sync orchestration logic
- `src/plugins.rs`: plugin model, discovery, lifecycle dispatch
- `src/error.rs`: shared app error types and `AppResult`

### Data Model (SQLite)

Schema includes:

- `notes`: note title/content/timestamps
- `tags`: unique tag names
- `note_tags`: many-to-many note-tag relationship
- `backlinks`: source/target note links
- `metadata`: key-value metadata per note
- `graph_edges`: generic graph relation edges
- `note_index` (FTS5): searchable title/content/tags index

### Local-first / Privacy-first

- Primary persistence is local SQLite.
- Cloud sync is represented by a sync state service and disabled by default.
- Local data remains authoritative by design.

## Plugin System

Plugins are discovered from the app plugin directory (`AppConfig.plugin_dir`), where each plugin is a directory containing `plugin.json`.

Current plugin runtime behavior:

- Discovery + manifest parsing
- Registration as `LoadedPlugin`
- Lifecycle event dispatch:
  - `AppStarted`
  - `NoteOpened(note_id)`
  - `NoteSaved(note_id)`
  - `SearchPerformed(query)`
- Sandbox boundary concept via plugin-root scoping (`sandbox_root`)

### Plugin Manifest Format

Create `plugin.json`:

```json
{
  "id": "example.quick-insert",
  "name": "Quick Insert",
  "version": "0.1.0",
  "description": "Insert reusable markdown snippets",
  "author": "You",
  "commands": [
    {
      "id": "insert-meeting-template",
      "title": "Insert Meeting Template",
      "snippet": "# Meeting\n\n## Agenda\n- "
    }
  ]
}
```

### Create a Plugin

1. Find your app data directory (platform-specific).
2. Create `plugins/<your-plugin-id>/plugin.json`.
3. Restart Nete.
4. Your plugin appears in the right-side plugin list.

## Notes for Developers

- This MVP emphasizes clean boundaries and extensibility while keeping the scope focused.
- The plugin system is intentionally scaffolded for safe incremental evolution.
- Graph layout is lightweight and deterministic for speed.


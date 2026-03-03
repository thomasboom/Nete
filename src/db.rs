use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppResult;
use crate::models::{GraphEdge, Note, NoteSummary, SearchResult, Tag};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> AppResult<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS note_tags (
                note_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY(note_id, tag_id),
                FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE,
                FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS backlinks (
                source_note_id INTEGER NOT NULL,
                target_note_id INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY(source_note_id, target_note_id),
                FOREIGN KEY(source_note_id) REFERENCES notes(id) ON DELETE CASCADE,
                FOREIGN KEY(target_note_id) REFERENCES notes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS metadata (
                note_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY(note_id, key),
                FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS graph_edges (
                source_note_id INTEGER NOT NULL,
                target_note_id INTEGER NOT NULL,
                relation TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                PRIMARY KEY(source_note_id, target_note_id, relation),
                FOREIGN KEY(source_note_id) REFERENCES notes(id) ON DELETE CASCADE,
                FOREIGN KEY(target_note_id) REFERENCES notes(id) ON DELETE CASCADE
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS note_index USING fts5(
                title,
                content,
                tags,
                note_id UNINDEXED,
                tokenize='porter unicode61'
            );

            CREATE INDEX IF NOT EXISTS idx_notes_updated_at ON notes(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_backlinks_target ON backlinks(target_note_id);
            CREATE INDEX IF NOT EXISTS idx_graph_edges_source ON graph_edges(source_note_id);
            "#,
        )?;

        self.rebuild_search_index()?;
        Ok(())
    }

    pub fn create_note(&self, title: &str) -> AppResult<i64> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO notes (title, content, created_at, updated_at) VALUES (?1, '', ?2, ?2)",
            params![title, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.reindex_note(id)?;
        Ok(id)
    }

    pub fn list_notes(&self) -> AppResult<Vec<NoteSummary>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, updated_at FROM notes ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |row| {
            let updated_raw: String = row.get(2)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_raw)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(NoteSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                updated_at,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_note(&self, note_id: i64) -> AppResult<Option<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, created_at, updated_at FROM notes WHERE id = ?1",
        )?;
        let note = stmt
            .query_row(params![note_id], |row| {
                let created_raw: String = row.get(3)?;
                let updated_raw: String = row.get(4)?;
                let created_at = DateTime::parse_from_rfc3339(&created_raw)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let updated_at = DateTime::parse_from_rfc3339(&updated_raw)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(Note {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    created_at,
                    updated_at,
                })
            })
            .optional()?;
        Ok(note)
    }

    pub fn save_note(&self, note_id: i64, title: &str, content: &str) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE notes SET title = ?1, content = ?2, updated_at = ?3 WHERE id = ?4",
            params![title, content, now, note_id],
        )?;

        self.rebuild_backlinks_for(note_id, content)?;
        self.reindex_note(note_id)?;
        Ok(())
    }

    pub fn upsert_tag(&self, tag_name: &str) -> AppResult<i64> {
        self.conn.execute(
            "INSERT INTO tags(name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            params![tag_name],
        )?;
        let mut stmt = self.conn.prepare("SELECT id FROM tags WHERE name = ?1")?;
        let id: i64 = stmt.query_row(params![tag_name], |row| row.get(0))?;
        Ok(id)
    }

    pub fn set_note_tags(&self, note_id: i64, tag_names: &[String]) -> AppResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM note_tags WHERE note_id = ?1", params![note_id])?;
        for tag in tag_names {
            tx.execute(
                "INSERT INTO tags(name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
                params![tag],
            )?;
            tx.execute(
                "INSERT INTO note_tags(note_id, tag_id)
                 SELECT ?1, id FROM tags WHERE name = ?2",
                params![note_id, tag],
            )?;
        }
        tx.commit()?;
        self.reindex_note(note_id)?;
        Ok(())
    }

    pub fn note_tags(&self, note_id: i64) -> AppResult<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name
             FROM tags t
             INNER JOIN note_tags nt ON nt.tag_id = t.id
             WHERE nt.note_id = ?1
             ORDER BY t.name ASC",
        )?;
        let rows = stmt.query_map(params![note_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn backlinks_to(&self, note_id: i64) -> AppResult<Vec<NoteSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.title, n.updated_at
             FROM notes n
             INNER JOIN backlinks b ON b.source_note_id = n.id
             WHERE b.target_note_id = ?1
             ORDER BY n.updated_at DESC",
        )?;
        let rows = stmt.query_map(params![note_id], |row| {
            let updated_raw: String = row.get(2)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_raw)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(NoteSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                updated_at,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn graph_edges(&self) -> AppResult<Vec<GraphEdge>> {
        let mut stmt = self.conn.prepare(
            "SELECT source_note_id, target_note_id, relation FROM graph_edges ORDER BY relation ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(GraphEdge {
                source_note_id: row.get(0)?,
                target_note_id: row.get(1)?,
                relation: row.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn search_notes(&self, query: &str) -> AppResult<Vec<SearchResult>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.title,
                    snippet(note_index, 1, '<b>', '</b>', ' … ', 12) as snip,
                    bm25(note_index) as rank
             FROM note_index
             INNER JOIN notes n ON n.id = note_index.note_id
             WHERE note_index MATCH ?1
             ORDER BY rank ASC
             LIMIT 64",
        )?;

        let rows = stmt.query_map(params![query], |row| {
            let score_float: f64 = row.get(3)?;
            Ok(SearchResult {
                note_id: row.get(0)?,
                title: row.get(1)?,
                snippet: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                score: (score_float * -1000.0) as i64,
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn all_tags(&self) -> AppResult<Vec<Tag>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM tags ORDER BY name COLLATE NOCASE ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn set_metadata(&self, note_id: i64, key: &str, value: &str) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO metadata(note_id, key, value) VALUES (?1, ?2, ?3)
             ON CONFLICT(note_id, key) DO UPDATE SET value = excluded.value",
            params![note_id, key, value],
        )?;
        Ok(())
    }

    fn rebuild_backlinks_for(&self, source_note_id: i64, content: &str) -> AppResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM backlinks WHERE source_note_id = ?1",
            params![source_note_id],
        )?;
        tx.execute(
            "DELETE FROM graph_edges WHERE source_note_id = ?1 AND relation = 'backlink'",
            params![source_note_id],
        )?;

        let mut target_lookup = tx.prepare("SELECT id FROM notes WHERE title = ?1 LIMIT 1")?;
        for link in extract_wikilinks(content) {
            if let Some(target_id) = target_lookup
                .query_row(params![link], |row| row.get::<_, i64>(0))
                .optional()?
            {
                tx.execute(
                    "INSERT OR IGNORE INTO backlinks(source_note_id, target_note_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![source_note_id, target_id, Utc::now().to_rfc3339()],
                )?;
                tx.execute(
                    "INSERT OR REPLACE INTO graph_edges(source_note_id, target_note_id, relation, weight)
                     VALUES (?1, ?2, 'backlink', 1.0)",
                    params![source_note_id, target_id],
                )?;
            }
        }
        drop(target_lookup);
        tx.commit()?;
        Ok(())
    }

    fn rebuild_search_index(&self) -> AppResult<()> {
        self.conn.execute("DELETE FROM note_index", [])?;
        let notes = self.list_notes()?;
        for note in notes {
            self.reindex_note(note.id)?;
        }
        Ok(())
    }

    fn reindex_note(&self, note_id: i64) -> AppResult<()> {
        let Some(note) = self.get_note(note_id)? else {
            return Ok(());
        };
        let tags = self
            .note_tags(note_id)?
            .into_iter()
            .map(|t| t.name)
            .collect::<Vec<_>>()
            .join(" ");

        self.conn
            .execute("DELETE FROM note_index WHERE note_id = ?1", params![note_id])?;

        self.conn.execute(
            "INSERT INTO note_index(title, content, tags, note_id) VALUES (?1, ?2, ?3, ?4)",
            params![note.title, note.content, tags, note_id],
        )?;
        Ok(())
    }
}

fn extract_wikilinks(input: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut cursor = 0usize;
    while let Some(start) = input[cursor..].find("[[") {
        let start_abs = cursor + start + 2;
        if let Some(end_rel) = input[start_abs..].find("]]") {
            let end_abs = start_abs + end_rel;
            let mut title = input[start_abs..end_abs].trim().to_string();
            if let Some(pipe) = title.find('|') {
                title.truncate(pipe);
            }
            if !title.is_empty() {
                links.push(title);
            }
            cursor = end_abs + 2;
        } else {
            break;
        }
    }
    links
}


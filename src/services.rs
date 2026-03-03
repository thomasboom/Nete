use chrono::{DateTime, Utc};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::db::Database;
use crate::error::AppResult;
use crate::models::{Note, NoteSummary, SearchResult};

#[derive(Debug, Clone)]
pub struct EditorBuffer {
    pub note_id: i64,
    pub title: String,
    pub content: String,
    pub tags: String,
    pub dirty: bool,
    pub last_saved_at: Option<DateTime<Utc>>,
}

impl EditorBuffer {
    pub fn from_note(note: &Note, tags: Vec<String>) -> Self {
        Self {
            note_id: note.id,
            title: note.title.clone(),
            content: note.content.clone(),
            tags: tags.join(", "),
            dirty: false,
            last_saved_at: Some(note.updated_at),
        }
    }

    pub fn parse_tags(&self) -> Vec<String> {
        self.tags
            .split(',')
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
            .collect()
    }
}

pub struct NoteService {
    matcher: SkimMatcherV2,
}

impl NoteService {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn create_note(&self, db: &Database, title: &str) -> AppResult<i64> {
        let title = if title.trim().is_empty() {
            format!("Untitled {}", Utc::now().format("%Y-%m-%d %H:%M"))
        } else {
            title.trim().to_string()
        };
        db.create_note(&title)
    }

    pub fn open_buffer(&self, db: &Database, note_id: i64) -> AppResult<Option<EditorBuffer>> {
        let Some(note) = db.get_note(note_id)? else {
            return Ok(None);
        };
        let tags = db
            .note_tags(note_id)?
            .into_iter()
            .map(|t| t.name)
            .collect::<Vec<_>>();
        Ok(Some(EditorBuffer::from_note(&note, tags)))
    }

    pub fn save_buffer(&self, db: &Database, buffer: &mut EditorBuffer) -> AppResult<()> {
        db.save_note(buffer.note_id, &buffer.title, &buffer.content)?;
        db.set_note_tags(buffer.note_id, &buffer.parse_tags())?;
        db.set_metadata(
            buffer.note_id,
            "word_count",
            &buffer.content.split_whitespace().count().to_string(),
        )?;
        buffer.dirty = false;
        buffer.last_saved_at = Some(Utc::now());
        Ok(())
    }

    pub fn search(
        &self,
        db: &Database,
        query: &str,
        notes: &[NoteSummary],
    ) -> AppResult<Vec<SearchResult>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut out = db.search_notes(query)?;

        for note in notes {
            if let Some(score) = self.matcher.fuzzy_match(&note.title, query) {
                if !out.iter().any(|x| x.note_id == note.id) {
                    out.push(SearchResult {
                        note_id: note.id,
                        title: note.title.clone(),
                        snippet: "title match".to_string(),
                        score,
                    });
                }
            }
        }

        out.sort_by(|a, b| b.score.cmp(&a.score));
        Ok(out)
    }
}

#[derive(Debug, Clone)]
pub struct SyncState {
    pub enabled: bool,
    pub last_status: String,
}

impl SyncState {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            last_status: "Cloud sync disabled (local-first mode)".into(),
        }
    }

    pub fn tick(&mut self) {
        if !self.enabled {
            self.last_status = "Cloud sync disabled (local data is authoritative)".into();
        }
    }
}


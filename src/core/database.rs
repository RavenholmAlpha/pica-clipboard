use crate::core::types::{Category, ClipboardItem, ClipboardType, Snippet};
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open database")?;

        // Enable WAL mode for concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.init_schema()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // History
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type INTEGER NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                source_app TEXT,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                is_pinned INTEGER DEFAULT 0
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_history_hash ON history(content_hash)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_history_created ON history(created_at DESC)",
            [],
        )?;

        // Categories
        conn.execute(
            "CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                icon TEXT,
                sort_order INTEGER DEFAULT 0,
                is_locked INTEGER DEFAULT 0
            )",
            [],
        )?;

        // Snippets
        conn.execute(
            "CREATE TABLE IF NOT EXISTS snippets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                is_masked INTEGER DEFAULT 0,
                usage_count INTEGER DEFAULT 0,
                updated_at INTEGER DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY(category_id) REFERENCES categories(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute_batch(
            "
            CREATE VIRTUAL TABLE IF NOT EXISTS history_fts USING fts5(
                content,
                source_app,
                content='history',
                content_rowid='id'
            );

            CREATE TRIGGER IF NOT EXISTS history_ai AFTER INSERT ON history BEGIN
              INSERT INTO history_fts(rowid, content, source_app) VALUES (new.id, new.content, new.source_app);
            END;

            CREATE TRIGGER IF NOT EXISTS history_ad AFTER DELETE ON history BEGIN
              INSERT INTO history_fts(history_fts, rowid, content, source_app) VALUES('delete', old.id, old.content, old.source_app);
            END;

            CREATE TRIGGER IF NOT EXISTS history_au AFTER UPDATE ON history BEGIN
              INSERT INTO history_fts(history_fts, rowid, content, source_app) VALUES('delete', old.id, old.content, old.source_app);
              INSERT INTO history_fts(rowid, content, source_app) VALUES (new.id, new.content, new.source_app);
            END;
            "
        ).context("Failed to init FTS")?;

        // Initialize default categories if empty
        let count: i64 = conn.query_row("SELECT count(*) FROM categories", [], |row| row.get(0))?;
        if count == 0 {
            conn.execute("INSERT INTO categories (name, icon, sort_order) VALUES ('General', '📝', 0)", [])?;
            conn.execute("INSERT INTO categories (name, icon, sort_order) VALUES ('Passwords', '🔑', 1)", [])?;
            conn.execute("INSERT INTO categories (name, icon, sort_order) VALUES ('Code', '💻', 2)", [])?;
        }

        Ok(())
    }

    // --- History Operations ---

    pub fn get_item_by_id(&self, id: i64) -> Result<Option<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, type, content, content_hash, source_app, created_at, is_pinned
             FROM history
             WHERE id = ?"
        )?;

        let mut rows = stmt.query_map(params![id], |row| self.row_to_clipboard_item(row))?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn insert_history(&self, item: &ClipboardItem) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        // Check for duplicates
        let existing: Option<i64> = conn.query_row(
            "SELECT id FROM history WHERE content_hash = ?",
            params![item.content_hash],
            |row| row.get(0),
        ).optional()?;

        if let Some(id) = existing {
            // Update timestamp
            conn.execute("UPDATE history SET created_at = strftime('%s', 'now') WHERE id = ?", params![id])?;
            return Ok(id);
        }

        conn.execute(
            "INSERT INTO history (type, content, content_hash, source_app, is_pinned) VALUES (?, ?, ?, ?, ?)",
            params![
                item.type_ as i64,
                item.content,
                item.content_hash,
                item.source_app,
                item.is_pinned
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_recent_history(&self, limit: usize, offset: usize) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, type, content, content_hash, source_app, created_at, is_pinned
             FROM history
             ORDER BY is_pinned DESC, created_at DESC
             LIMIT ? OFFSET ?"
        )?;

        let rows = stmt.query_map(params![limit, offset], |row| self.row_to_clipboard_item(row))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn search_history(&self, query: &str) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        // Use FTS
        let mut stmt = conn.prepare(
            "SELECT id, type, content, content_hash, source_app, created_at, is_pinned
             FROM history
             WHERE id IN (SELECT rowid FROM history_fts WHERE history_fts MATCH ? ORDER BY rank)
             ORDER BY is_pinned DESC, created_at DESC
             LIMIT 50"
        )?;

        let rows = stmt.query_map(params![query], |row| self.row_to_clipboard_item(row))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn delete_history(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM history WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn toggle_pin(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let current: bool = conn.query_row("SELECT is_pinned FROM history WHERE id = ?", params![id], |row| row.get(0))?;
        let new_state = !current;
        conn.execute("UPDATE history SET is_pinned = ? WHERE id = ?", params![new_state, id])?;
        Ok(new_state)
    }

    // --- Snippets Operations ---

    pub fn get_categories(&self) -> Result<Vec<Category>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, icon, sort_order, is_locked FROM categories ORDER BY sort_order ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                icon: row.get(2)?,
                sort_order: row.get(3)?,
                is_locked: row.get(4)?,
            })
        })?;

        let mut cats = Vec::new();
        for row in rows {
            cats.push(row?);
        }
        Ok(cats)
    }

    pub fn get_snippets(&self, category_id: i64) -> Result<Vec<Snippet>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, category_id, title, content, is_masked, usage_count, updated_at
             FROM snippets
             WHERE category_id = ?
             ORDER BY usage_count DESC, updated_at DESC"
        )?;

        let rows = stmt.query_map(params![category_id], |row| {
            Ok(Snippet {
                id: Some(row.get(0)?),
                category_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                is_masked: row.get(4)?,
                usage_count: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut snippets = Vec::new();
        for row in rows {
            snippets.push(row?);
        }
        Ok(snippets)
    }

    pub fn add_snippet(&self, snippet: &Snippet) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO snippets (category_id, title, content, is_masked, usage_count) VALUES (?, ?, ?, ?, ?)",
            params![
                snippet.category_id,
                snippet.title,
                snippet.content,
                snippet.is_masked,
                snippet.usage_count
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    // --- Helper ---

    fn row_to_clipboard_item(&self, row: &Row) -> rusqlite::Result<ClipboardItem> {
        Ok(ClipboardItem {
            id: Some(row.get(0)?),
            type_: ClipboardType::from(row.get::<_, i64>(1)?),
            content: row.get(2)?,
            content_hash: row.get(3)?,
            source_app: row.get(4)?,
            created_at: row.get(5)?,
            is_pinned: row.get(6)?,
            tags: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_init() {
        let db = Database::open_in_memory().unwrap();
        let cats = db.get_categories().unwrap();
        assert_eq!(cats.len(), 3);
    }

    #[test]
    fn test_history_crud() {
        let db = Database::open_in_memory().unwrap();
        let item = ClipboardItem {
            id: None,
            type_: ClipboardType::Text,
            content: "Hello".to_string(),
            content_hash: "hash123".to_string(),
            source_app: Some("test".to_string()),
            created_at: 0,
            is_pinned: false,
            tags: vec![],
        };

        let id = db.insert_history(&item).unwrap();
        assert!(id > 0);

        let recent = db.get_recent_history(10, 0).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content, "Hello");

        let id2 = db.insert_history(&item).unwrap();
        assert_eq!(id, id2);

        let search = db.search_history("Hello").unwrap();
        assert_eq!(search.len(), 1);

        db.delete_history(id).unwrap();
        let recent_after = db.get_recent_history(10, 0).unwrap();
        assert_eq!(recent_after.len(), 0);
    }

    #[test]
    fn test_snippet_crud() {
        let db = Database::open_in_memory().unwrap();
        let cats = db.get_categories().unwrap();
        let cat_id = cats[0].id.unwrap();

        let snip = Snippet {
            id: None,
            category_id: cat_id,
            title: "My Snippet".to_string(),
            content: "Secret".to_string(),
            is_masked: true,
            usage_count: 0,
            updated_at: 0,
        };

        let _id = db.add_snippet(&snip).unwrap();

        let snippets = db.get_snippets(cat_id).unwrap();
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].title, "My Snippet");
    }
}

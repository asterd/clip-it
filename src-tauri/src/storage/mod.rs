use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

use crate::settings::Settings;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub id: i64,
    pub created_at: i64,
    pub kind: String,
    pub text: String,
    pub preview_text: String,
    pub image_width: Option<i64>,
    pub image_height: Option<i64>,
    pub favorite: bool,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub total: u32,
    pub items: Vec<SearchItem>,
}

#[derive(Debug)]
pub struct ClipboardPayload {
    pub kind: String,
    pub text: Option<String>,
    pub image_rgba: Option<Vec<u8>>,
    pub image_width: Option<i64>,
    pub image_height: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPreview {
    pub kind: String,
    pub text: String,
    pub image_rgba: Option<Vec<u8>>,
    pub image_width: Option<i64>,
    pub image_height: Option<i64>,
}

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("failed to create db parent directory")?;
        }

        let conn = Connection::open(path).context("failed to open sqlite db")?;
        let mut storage = Self { conn };
        storage.run_migrations()?;
        Ok(storage)
    }

    pub fn run_migrations(&mut self) -> Result<()> {
        self.conn
            .execute_batch(include_str!("migrations/001_init.sql"))
            .context("failed to run migrations")?;
        self.ensure_item_columns()?;
        Ok(())
    }

    fn ensure_item_columns(&self) -> Result<()> {
        let mut cols = HashSet::new();
        let mut stmt = self.conn.prepare("PRAGMA table_info(items)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for r in rows {
            cols.insert(r?);
        }

        if !cols.contains("image_rgba") {
            self.conn
                .execute("ALTER TABLE items ADD COLUMN image_rgba BLOB", [])?;
        }
        if !cols.contains("image_width") {
            self.conn
                .execute("ALTER TABLE items ADD COLUMN image_width INTEGER", [])?;
        }
        if !cols.contains("image_height") {
            self.conn
                .execute("ALTER TABLE items ADD COLUMN image_height INTEGER", [])?;
        }
        if !cols.contains("favorite") {
            self.conn
                .execute("ALTER TABLE items ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0", [])?;
        }
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_items_favorite_deleted ON items(favorite, deleted)",
            [],
        )?;

        Ok(())
    }

    pub fn load_settings(&self) -> Result<Settings> {
        let mut settings = Settings::default();
        let mut stmt = self
            .conn
            .prepare("SELECT key, value_json FROM settings")
            .context("failed to prepare settings query")?;

        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        })?;

        for row in rows {
            let (key, value_json) = row?;
            let value: Value = serde_json::from_str(&value_json).unwrap_or(Value::Null);
            apply_setting_value(&mut settings, &key, value);
        }

        Ok(settings)
    }

    pub fn upsert_setting(&self, key: &str, value: &Value) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value_json) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
            params![key, value.to_string()],
        )?;
        Ok(())
    }

    pub fn last_fingerprint(&self) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT fingerprint FROM items
             WHERE deleted = 0
             ORDER BY created_at DESC
             LIMIT 1",
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn insert_item(
        &self,
        kind: &str,
        text: Option<&str>,
        fingerprint: &str,
        image_rgba: Option<&[u8]>,
        image_width: Option<i64>,
        image_height: Option<i64>,
    ) -> Result<i64> {
        let now = unix_ms();
        self.conn.execute(
            "INSERT INTO items(created_at, kind, text, fingerprint, image_rgba, image_width, image_height, favorite, pinned, deleted)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, 0)",
            params![now, kind, text, fingerprint, image_rgba, image_width, image_height],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn enforce_max_items(&self, max_items: i64) -> Result<()> {
        if max_items <= 0 {
            return Ok(());
        }

        self.conn.execute(
            "UPDATE items
             SET deleted = 1
             WHERE id IN (
               SELECT id
               FROM items
               WHERE deleted = 0 AND pinned = 0 AND favorite = 0
               ORDER BY created_at DESC
               LIMIT -1 OFFSET ?1
             )",
            params![max_items],
        )?;

        Ok(())
    }

    pub fn get_item_clipboard_payload(&self, item_id: i64) -> Result<Option<ClipboardPayload>> {
        self.conn
            .query_row(
                "SELECT kind, text, image_rgba, image_width, image_height
                 FROM items
                 WHERE id = ?1 AND deleted = 0
                 LIMIT 1",
                params![item_id],
                |row| {
                    Ok(ClipboardPayload {
                        kind: row.get(0)?,
                        text: row.get(1)?,
                        image_rgba: row.get(2)?,
                        image_width: row.get(3)?,
                        image_height: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_item_preview(&self, item_id: i64) -> Result<Option<ItemPreview>> {
        self.conn
            .query_row(
                "SELECT kind, COALESCE(text, ''), image_rgba, image_width, image_height
                 FROM items
                 WHERE id = ?1 AND deleted = 0
                 LIMIT 1",
                params![item_id],
                |row| {
                    Ok(ItemPreview {
                        kind: row.get(0)?,
                        text: row.get(1)?,
                        image_rgba: row.get(2)?,
                        image_width: row.get(3)?,
                        image_height: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn search_items(
        &self,
        query: &str,
        limit: u32,
        offset: u32,
        filter: &str,
    ) -> Result<SearchResponse> {
        let capped_limit = limit.clamp(1, 200);
        let q = query.trim();
        let filter = match filter {
            "favorites" | "pinned" => filter,
            _ => "all",
        };

        if q.is_empty() {
            let mut stmt = self.conn.prepare(
                "SELECT id, created_at, kind, COALESCE(text, ''), image_width, image_height, favorite, pinned
                 FROM items
                 WHERE deleted = 0
                   AND (?3 = 'all' OR (?3 = 'favorites' AND favorite = 1) OR (?3 = 'pinned' AND pinned = 1))
                 ORDER BY pinned DESC, favorite DESC, created_at DESC
                 LIMIT ?1 OFFSET ?2",
            )?;

            let rows = stmt.query_map(params![capped_limit, offset, filter], |row| {
                let kind: String = row.get(2)?;
                let text: String = row.get(3)?;
                let w: Option<i64> = row.get(4)?;
                let h: Option<i64> = row.get(5)?;
                Ok(SearchItem {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    kind: kind.clone(),
                    preview_text: preview_text(&kind, &text),
                    text,
                    image_width: w,
                    image_height: h,
                    favorite: row.get::<_, i64>(6)? == 1,
                    pinned: row.get::<_, i64>(7)? == 1,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }

            let total: u32 = self.conn.query_row(
                "SELECT COUNT(*) FROM items
                 WHERE deleted = 0
                   AND (?1 = 'all' OR (?1 = 'favorites' AND favorite = 1) OR (?1 = 'pinned' AND pinned = 1))",
                params![filter],
                |r| r.get(0),
            )?;

            return Ok(SearchResponse { total, items });
        }

        let match_query = format!("\"{}\"*", q.replace('"', " "));

        let total: u32 = self.conn.query_row(
            "SELECT COUNT(*)
             FROM items_fts f
             JOIN items i ON i.id = f.rowid
             WHERE f.text MATCH ?1 AND i.deleted = 0
               AND (?2 = 'all' OR (?2 = 'favorites' AND i.favorite = 1) OR (?2 = 'pinned' AND i.pinned = 1))",
            params![match_query, filter],
            |r| r.get(0),
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT i.id, i.created_at, i.kind, COALESCE(i.text, ''), i.image_width, i.image_height, i.favorite, i.pinned
             FROM items_fts f
             JOIN items i ON i.id = f.rowid
             WHERE f.text MATCH ?1 AND i.deleted = 0
               AND (?4 = 'all' OR (?4 = 'favorites' AND i.favorite = 1) OR (?4 = 'pinned' AND i.pinned = 1))
             ORDER BY i.pinned DESC, i.favorite DESC, i.created_at DESC
             LIMIT ?2 OFFSET ?3",
        )?;

        let rows = stmt.query_map(params![match_query, capped_limit, offset, filter], |row| {
            let kind: String = row.get(2)?;
            let text: String = row.get(3)?;
            let w: Option<i64> = row.get(4)?;
            let h: Option<i64> = row.get(5)?;
            Ok(SearchItem {
                id: row.get(0)?,
                created_at: row.get(1)?,
                kind: kind.clone(),
                preview_text: preview_text(&kind, &text),
                text,
                image_width: w,
                image_height: h,
                favorite: row.get::<_, i64>(6)? == 1,
                pinned: row.get::<_, i64>(7)? == 1,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }

        Ok(SearchResponse { total, items })
    }

    pub fn set_favorite(&self, item_id: i64, favorite: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE items SET favorite = ?1 WHERE id = ?2",
            params![if favorite { 1 } else { 0 }, item_id],
        )?;
        Ok(())
    }

    pub fn pin_item(&self, item_id: i64, pinned: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE items SET pinned = ?1 WHERE id = ?2",
            params![if pinned { 1 } else { 0 }, item_id],
        )?;
        Ok(())
    }

    pub fn delete_item(&self, item_id: i64) -> Result<()> {
        self.conn
            .execute("UPDATE items SET deleted = 1 WHERE id = ?1", params![item_id])?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<()> {
        self.conn
            .execute("UPDATE items SET deleted = 1 WHERE pinned = 0 AND favorite = 0", [])?;
        Ok(())
    }

    pub fn clear_all_history(&self) -> Result<()> {
        self.conn
            .execute("UPDATE items SET deleted = 1 WHERE deleted = 0", [])?;
        Ok(())
    }
}

pub fn apply_setting_value(settings: &mut Settings, key: &str, value: Value) {
    match key {
        "hotkey" => {
            if let Some(v) = value.as_str() {
                settings.hotkey = v.to_string();
            }
        }
        "blur_close" => {
            if let Some(v) = value.as_bool() {
                settings.blur_close = v;
            }
        }
        "polling_interval_ms" => {
            if let Some(v) = value.as_u64() {
                settings.polling_interval_ms = v.clamp(100, 5000);
            }
        }
        "capture_enabled" => {
            if let Some(v) = value.as_bool() {
                settings.capture_enabled = v;
            }
        }
        "max_items" => {
            if let Some(v) = value.as_i64() {
                settings.max_items = v.clamp(10, 5000);
            }
        }
        "window_opacity" => {
            if let Some(v) = value.as_i64() {
                settings.window_opacity = v.clamp(35, 100);
            }
        }
        "colored_icons" => {
            if let Some(v) = value.as_bool() {
                settings.colored_icons = v;
            }
        }
        _ => {}
    }
}

fn preview_text(kind: &str, text: &str) -> String {
    if kind == "image" {
        return "Image".to_string();
    }

    let compact = text.replace('\n', " ").replace('\r', " ");
    let mut out = compact.chars().take(140).collect::<String>();
    if compact.chars().count() > 140 {
        out.push_str("...");
    }
    out
}

fn unix_ms() -> i64 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    dur.as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::Storage;

    #[test]
    fn search_filter_favorites_and_pinned() {
        let db_path = std::env::temp_dir().join(format!(
            "clipit-test-{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));

        let storage = Storage::open(&db_path).expect("open db");
        let a = storage
            .insert_item("text", Some("alpha"), "fp-a", None, None, None)
            .expect("insert a");
        let b = storage
            .insert_item("text", Some("beta"), "fp-b", None, None, None)
            .expect("insert b");

        storage.set_favorite(a, true).expect("favorite a");
        storage.pin_item(b, true).expect("pin b");

        let fav = storage
            .search_items("", 50, 0, "favorites")
            .expect("search favorites");
        assert_eq!(fav.items.len(), 1);
        assert_eq!(fav.items[0].id, a);

        let pinned = storage
            .search_items("", 50, 0, "pinned")
            .expect("search pinned");
        assert_eq!(pinned.items.len(), 1);
        assert_eq!(pinned.items[0].id, b);

        let _ = std::fs::remove_file(db_path);
    }
}

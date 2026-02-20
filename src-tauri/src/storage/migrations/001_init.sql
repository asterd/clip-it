PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  created_at INTEGER NOT NULL,
  kind TEXT NOT NULL DEFAULT 'text',
  text TEXT,
  fingerprint TEXT NOT NULL,
  image_rgba BLOB,
  image_width INTEGER,
  image_height INTEGER,
  favorite INTEGER NOT NULL DEFAULT 0,
  pinned INTEGER NOT NULL DEFAULT 0,
  deleted INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_items_created_at_desc ON items(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_items_pinned_deleted ON items(pinned, deleted);
CREATE INDEX IF NOT EXISTS idx_items_fingerprint ON items(fingerprint);

CREATE VIRTUAL TABLE IF NOT EXISTS items_fts
USING fts5(text, content='items', content_rowid='id');

CREATE TRIGGER IF NOT EXISTS items_ai AFTER INSERT ON items BEGIN
  INSERT INTO items_fts(rowid, text) VALUES (new.id, COALESCE(new.text, ''));
END;

CREATE TRIGGER IF NOT EXISTS items_ad AFTER DELETE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, text) VALUES('delete', old.id, COALESCE(old.text, ''));
END;

CREATE TRIGGER IF NOT EXISTS items_au AFTER UPDATE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, text) VALUES('delete', old.id, COALESCE(old.text, ''));
  INSERT INTO items_fts(rowid, text) VALUES (new.id, COALESCE(new.text, ''));
END;

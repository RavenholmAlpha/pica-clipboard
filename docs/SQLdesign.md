# Database Schema Design (SQLite)

## 1. Overview
数据库文件位置：`~/.picaclip/data.db` (Windows: `%APPDATA%\picaclip\data.db`).
必须启用 SQLite 扩展：`FTS5` (Full-Text Search)。

## 2. Tables Definition

### 2.1 Table: `history`
存储剪贴板历史记录。

```sql
CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    -- Content Types: 1=Text, 2=Image(Path), 3=File(Path)
    type INTEGER NOT NULL,
    
    -- Actual content or file path
    content TEXT NOT NULL,
    
    -- Hash for deduplication (SHA256 hex string)
    content_hash TEXT NOT NULL,
    
    -- Source application name (e.g., "chrome.exe")
    source_app TEXT,
    
    -- Creation timestamp (Unix Epoch in seconds)
    created_at INTEGER DEFAULT (strftime('%s', 'now')),
    
    -- Mark as pinned/favorite (0=false, 1=true)
    is_pinned INTEGER DEFAULT 0
);

-- Index for deduplication check (Critical for performance)
CREATE INDEX IF NOT EXISTS idx_history_hash ON history(content_hash);

-- Index for time-based query (Recent items first)
CREATE INDEX IF NOT EXISTS idx_history_created ON history(created_at DESC);
```

### 2.2 Table: `categories`
用于 `Snippets` 模块的分类管理。

```sql
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    -- Display name
    name TEXT NOT NULL,
    
    -- Icon name or unicode character
    icon TEXT,
    
    -- Sort order in the UI
    sort_order INTEGER DEFAULT 0,
    
    -- Is this category locked? (Requires auth to view items)
    is_locked INTEGER DEFAULT 0,
    
    created_at INTEGER DEFAULT (strftime('%s', 'now'))
);

-- Pre-populate default categories
INSERT INTO categories (name, icon, sort_order) VALUES ('General', '📝', 0);
INSERT INTO categories (name, icon, sort_order) VALUES ('Passwords', '🔑', 1);
INSERT INTO categories (name, icon, sort_order) VALUES ('Code', '💻', 2);
```

### 2.3 Table: `tags`
标签管理。

```sql
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT, -- Hex color code for UI
    created_at INTEGER DEFAULT (strftime('%s', 'now'))
);
```

### 2.4 Table: `item_tags`
关联表，支持 History 和 Snippets 的多标签功能。

```sql
CREATE TABLE IF NOT EXISTS item_tags (
    tag_id INTEGER NOT NULL,
    
    -- Target ID (history.id or snippets.id)
    item_id INTEGER NOT NULL,
    
    -- Target Type: 1=History, 2=Snippet
    item_type INTEGER NOT NULL,
    
    PRIMARY KEY (tag_id, item_id, item_type),
    FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_item_tags_target ON item_tags(item_id, item_type);
```

### 2.5 Table: `snippets`
结构化便签数据。

```sql
CREATE TABLE IF NOT EXISTS snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    category_id INTEGER NOT NULL,
    
    -- Title for the snippet
    title TEXT NOT NULL,
    
    -- The content to be pasted (Maybe encrypted if is_masked=1)
    content TEXT NOT NULL,
    
    -- 0=Plain, 1=Masked/Encrypted
    is_masked INTEGER DEFAULT 0,
    
    usage_count INTEGER DEFAULT 0,
    updated_at INTEGER DEFAULT (strftime('%s', 'now')),
    
    FOREIGN KEY(category_id) REFERENCES categories(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_snippets_cat ON snippets(category_id);
```

## 3. Full-Text Search (FTS5)

为了实现统一且高效的搜索，我们需要建立 FTS 虚拟表。由于 `history` 表可能很大，直接对主表做 FTS 可能会增加写入开销，但考虑到读取频率远高于写入，且为了搜索性能，建议使用 `FTS5` `content` 选项或触发器维护。

这里采用 **External Content FTS** 模式，减少存储冗余。

### 3.1 FTS Table: `search_index`
统一索引 `history` 和 `snippets`。

```sql
-- Virtual table for searching history
CREATE VIRTUAL TABLE IF NOT EXISTS history_fts USING fts5(
    content, 
    source_app, 
    content='history', 
    content_rowid='id'
);

-- Triggers to keep history_fts in sync
CREATE TRIGGER history_ai AFTER INSERT ON history BEGIN
  INSERT INTO history_fts(rowid, content, source_app) VALUES (new.id, new.content, new.source_app);
END;

CREATE TRIGGER history_ad AFTER DELETE ON history BEGIN
  INSERT INTO history_fts(history_fts, rowid, content, source_app) VALUES('delete', old.id, old.content, old.source_app);
END;

CREATE TRIGGER history_au AFTER UPDATE ON history BEGIN
  INSERT INTO history_fts(history_fts, rowid, content, source_app) VALUES('delete', old.id, old.content, old.source_app);
  INSERT INTO history_fts(rowid, content, source_app) VALUES (new.id, new.content, new.source_app);
END;

-- Virtual table for searching snippets
CREATE VIRTUAL TABLE IF NOT EXISTS snippets_fts USING fts5(
    title, 
    content, 
    content='snippets', 
    content_rowid='id'
);
-- (Similar triggers needed for snippets if content is not encrypted. 
--  If content IS encrypted, we should NOT index the 'content' column in FTS, only 'title'.)
```

## 4. Query Examples

### 4.1 Insert new history item
```sql
INSERT INTO history (type, content, content_hash, source_app) 
VALUES (1, 'Hello World', 'a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e', 'notepad.exe');
```

### 4.2 Search all (Unified Search)
```sql
-- Search History (Joined with tags for display, but search logic might be separate or combined)
-- Note: To search by tag name using FTS, we might need to include tag names in the FTS table or use a separate query.
-- A simple approach for "Search by content AND tags":
-- SELECT ... FROM history_fts ... WHERE ...
-- UNION
-- SELECT ... FROM history h JOIN item_tags it ON h.id=it.item_id JOIN tags t ON it.tag_id=t.id WHERE t.name LIKE '%query%'

SELECT id, content, 'history' as source FROM history_fts WHERE history_fts MATCH 'search_query' ORDER BY rank LIMIT 20;

-- UNION ALL

-- Search Snippets
SELECT id, content, 'snippet' as source FROM snippets_fts WHERE snippets_fts MATCH 'search_query' ORDER BY rank LIMIT 20;
```

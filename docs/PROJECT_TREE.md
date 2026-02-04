# Project Directory Structure

## 1. Root Directory
```
picaclip/
├── Cargo.toml          # Workspace or Single Crate definition
├── build.rs            # Build script for Slint
├── README.md
├── docs/               # Documentation
│   ├── PRD.md
│   ├── ARCHITECTURE.md
│   └── SQLdesign.md
└── src/                # Source Code
```

## 2. Source Code (`src/`)

```
src/
├── main.rs             # Application Entry Point (Setup Tokio & Slint)
├── app.slint           # Main UI definition file
├── core/               # Backend Logic (Headless)
│   ├── mod.rs
│   ├── clipboard.rs    # Clipboard Monitor (arboard wrapper)
│   ├── database.rs     # SQLite connection & query helpers
│   ├── manager.rs      # Core Service Logic (Controller)
│   └── types.rs        # Shared structs (ClipboardItem, Snippet, etc.)
├── ui/                 # UI Logic (Slint Code-behind)
│   ├── mod.rs
│   ├── window.rs       # Window behavior & event handling
│   └── viewmodels.rs   # Data models adapted for Slint
└── utils/              # Helper Modules
    ├── mod.rs
    ├── crypto.rs       # AES-GCM encryption/decryption
    ├── text.rs         # Regex, text cleaning
    └── paths.rs        # App data paths resolution
```

## 3. Module Responsibilities

### `src/main.rs`
- Initialize `env_logger`.
- Initialize `SQLite` connection pool.
- Spawn `tokio::runtime`.
- Create `Slint` window instance.
- Start `Core Manager` loop.
- Launch `Slint` event loop.

### `src/core/`
- **clipboard.rs**: 使用 `arboard` 监听剪贴板，通过 `mpsc::Sender` 发送 `ClipboardChange` 事件。
- **database.rs**: 封装 `rusqlite` 或 `sqlx`，提供 `insert_history`, `search_items` 等原子操作。
- **manager.rs**: 核心状态机。接收 UI 命令 (Paste, Delete)，接收 Clipboard 事件，协调 DB 和 UI 更新。

### `src/ui/`
- **app.slint**: 定义界面布局 (ListView, TextInput, Sidebar)。定义 `callback` 和 `property`。
- **window.rs**: Rust 侧实现 Slint 的回调，如 `on_paste_request`。

### `src/utils/`
- **crypto.rs**: 处理 Snippet 内容的加解密。
- **text.rs**: 实现 `is_sensitive()` 判断逻辑（正则匹配）。

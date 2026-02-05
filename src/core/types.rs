use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClipboardType {
    Text = 1,
    Image = 2,
    File = 3,
}

impl From<i64> for ClipboardType {
    fn from(value: i64) -> Self {
        match value {
            1 => ClipboardType::Text,
            2 => ClipboardType::Image,
            3 => ClipboardType::File,
            _ => ClipboardType::Text, // Default
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: Option<i64>,
    pub type_: ClipboardType,
    pub content: String, // Text content or File Path
    pub content_hash: String,
    pub source_app: Option<String>,
    pub created_at: i64,
    pub is_pinned: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Option<i64>,
    pub name: String,
    pub icon: Option<String>,
    pub sort_order: i64,
    pub is_locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: Option<i64>,
    pub category_id: i64,
    pub title: String,
    pub content: String,
    pub is_masked: bool,
    pub usage_count: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    PasteItem(i64),
    DeleteHistory(i64),
    TogglePin(i64),
    Search(String),
    AddSnippet(Snippet),
    ToggleQueueMode(bool),
    NextQueueItem,
    Exit,
}

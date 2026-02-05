use crate::core::types::{ClipboardItem, ClipboardType};
use crate::utils::{paths, text};
use anyhow::{Context, Result};
use arboard::{Clipboard, ImageData};
use chrono::Utc;
use image::{ImageBuffer, RgbaImage};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct ClipboardMonitor {
    tx: mpsc::Sender<ClipboardItem>,
    clipboard: Arc<Mutex<Clipboard>>,
    last_hash: String,
}

impl ClipboardMonitor {
    pub fn new(tx: mpsc::Sender<ClipboardItem>) -> Result<Self> {
        let clipboard = Clipboard::new().context("Failed to initialize clipboard")?;
        Ok(Self {
            tx,
            clipboard: Arc::new(Mutex::new(clipboard)),
            last_hash: String::new(),
        })
    }

    pub async fn run(mut self) {
        log::info!("Starting Clipboard Monitor");

        loop {
            // Poll interval
            tokio::time::sleep(Duration::from_millis(500)).await;

            if let Err(e) = self.check_clipboard() {
                log::error!("Error checking clipboard: {}", e);
            }
        }
    }

    fn check_clipboard(&mut self) -> Result<()> {
        let mut clipboard = self.clipboard.lock().unwrap();

        // Try getting text
        if let Ok(text) = clipboard.get_text() {
            let cleaned = text::clean_text(&text);
            if cleaned.is_empty() {
                return Ok(());
            }

            let hash = self.compute_hash(cleaned.as_bytes());
            if hash != self.last_hash {
                // New content
                self.last_hash = hash.clone();

                let item = ClipboardItem {
                    id: None,
                    type_: ClipboardType::Text,
                    content: cleaned,
                    content_hash: hash,
                    source_app: None,
                    created_at: Utc::now().timestamp(),
                    is_pinned: false,
                    tags: Vec::new(),
                };

                let _ = self.tx.try_send(item);
            }
            return Ok(());
        }

        // Try getting image
        if let Ok(image_data) = clipboard.get_image() {
            // Convert to hashable bytes
            let hash = self.compute_hash(&image_data.bytes);

            if hash != self.last_hash {
                self.last_hash = hash.clone();

                // Save image to file
                let path = self.save_image(&image_data)?;

                let item = ClipboardItem {
                    id: None,
                    type_: ClipboardType::Image,
                    content: path, // Path to file
                    content_hash: hash,
                    source_app: None,
                    created_at: Utc::now().timestamp(),
                    is_pinned: false,
                    tags: Vec::new(),
                };

                let _ = self.tx.try_send(item);
            }
            return Ok(());
        }

        Ok(())
    }

    fn compute_hash(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    fn save_image(&self, image_data: &ImageData) -> Result<String> {
        let width = image_data.width as u32;
        let height = image_data.height as u32;
        // Arboard returns RGBA8888
        let img: RgbaImage = ImageBuffer::from_raw(width, height, image_data.bytes.clone().into_owned())
            .context("Failed to create image buffer")?;

        let filename = format!("img_{}.png", Utc::now().format("%Y%m%d_%H%M%S"));
        let path = paths::get_image_cache_dir().join(filename);

        img.save(&path).context("Failed to save image to disk")?;

        Ok(path.to_string_lossy().to_string())
    }
}

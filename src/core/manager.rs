use crate::core::database::Database;
use crate::core::types::{AppCommand, ClipboardItem, ClipboardType};
use enigo::{Enigo, Key, Keyboard, Settings, Direction};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc;

// Trait for UI updates
pub trait UiHandle: Send + Sync {
    fn update_history(&self, items: Vec<ClipboardItem>);
    fn update_search_results(&self, items: Vec<ClipboardItem>);
    fn hide_window(&self);
    fn show_notification(&self, msg: String);
}

pub struct Manager {
    db: Database,
    clipboard_rx: mpsc::Receiver<ClipboardItem>,
    command_rx: mpsc::Receiver<AppCommand>,
    ui: Arc<dyn UiHandle>,
    paste_queue: VecDeque<ClipboardItem>,
    is_queue_mode: bool,
    enigo: Enigo,
}

impl Manager {
    pub fn new(
        db: Database,
        clipboard_rx: mpsc::Receiver<ClipboardItem>,
        command_rx: mpsc::Receiver<AppCommand>,
        ui: Arc<dyn UiHandle>,
    ) -> Self {
        Self {
            db,
            clipboard_rx,
            command_rx,
            ui,
            paste_queue: VecDeque::new(),
            is_queue_mode: false,
            enigo: Enigo::new(&Settings::default()).expect("Failed to init enigo"),
        }
    }

    pub async fn run(mut self) {
        log::info!("Starting Core Manager");

        // Load initial history
        if let Ok(items) = self.db.get_recent_history(50, 0) {
            self.ui.update_history(items);
        }

        loop {
            tokio::select! {
                Some(item) = self.clipboard_rx.recv() => {
                    self.handle_clipboard_update(item).await;
                }
                Some(cmd) = self.command_rx.recv() => {
                    if let AppCommand::Exit = cmd {
                        break;
                    }
                    self.handle_command(cmd).await;
                }
            }
        }
    }

    async fn handle_clipboard_update(&mut self, item: ClipboardItem) {
        if self.is_queue_mode {
            if let Ok(_id) = self.db.insert_history(&item) {
                 self.paste_queue.push_back(item.clone());
                 self.ui.show_notification(format!("Added to queue. Size: {}", self.paste_queue.len()));

                 self.refresh_history();
            }
        } else {
             if let Err(e) = self.db.insert_history(&item) {
                 log::error!("Failed to save history: {}", e);
             } else {
                 self.refresh_history();
             }
        }
    }

    async fn handle_command(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::PasteItem(id) => {
                if let Ok(Some(item)) = self.db.get_item_by_id(id) {
                     self.perform_paste(item);
                }
            }
            AppCommand::DeleteHistory(id) => {
                let _ = self.db.delete_history(id);
                self.refresh_history();
            }
            AppCommand::TogglePin(id) => {
                let _ = self.db.toggle_pin(id);
                self.refresh_history();
            }
            AppCommand::Search(query) => {
                if query.is_empty() {
                    self.refresh_history();
                } else {
                    if let Ok(results) = self.db.search_history(&query) {
                        self.ui.update_search_results(results);
                    }
                }
            }
            AppCommand::ToggleQueueMode(enabled) => {
                self.is_queue_mode = enabled;
                if !enabled {
                    self.paste_queue.clear();
                }
                self.ui.show_notification(format!("Queue Mode: {}", enabled));
            }
            AppCommand::AddSnippet(snip) => {
                let _ = self.db.add_snippet(&snip);
            }
            AppCommand::NextQueueItem => {
                if let Some(item) = self.paste_queue.pop_front() {
                    self.perform_paste(item);
                    self.ui.show_notification(format!("Queue Size: {}", self.paste_queue.len()));
                } else {
                    self.ui.show_notification("Queue empty".to_string());
                }
            }
            AppCommand::Exit => {}
        }
    }

    fn perform_paste(&mut self, item: ClipboardItem) {
        self.ui.hide_window();
        std::thread::sleep(std::time::Duration::from_millis(100));

        if let Ok(mut clipboard) = arboard::Clipboard::new() {
             match item.type_ {
                 ClipboardType::Image => {
                     let path = std::path::Path::new(&item.content);
                     if let Ok(img) = image::open(path) {
                         let rgba8 = img.to_rgba8();
                         let img_data = arboard::ImageData {
                             width: rgba8.width() as usize,
                             height: rgba8.height() as usize,
                             bytes: std::borrow::Cow::Owned(rgba8.into_vec()),
                         };
                         let _ = clipboard.set_image(img_data);
                         log::info!("Pasted image from {}", item.content);
                     } else {
                         log::error!("Failed to load image from {}", item.content);
                     }
                 },
                 _ => {
                     let _ = clipboard.set_text(item.content.clone());
                 }
             }
        }

        log::info!("Simulating paste input");

        let _ = self.enigo.key(Key::Control, Direction::Press);
        let _ = self.enigo.key(Key::Unicode('v'), Direction::Click);
        let _ = self.enigo.key(Key::Control, Direction::Release);
    }

    fn refresh_history(&self) {
        if let Ok(items) = self.db.get_recent_history(50, 0) {
             self.ui.update_history(items);
        }
    }
}

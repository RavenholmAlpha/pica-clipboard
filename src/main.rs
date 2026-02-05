mod core;
mod utils;
mod ui;

use crate::core::database::Database;
use crate::core::manager::{Manager, UiHandle};
use crate::core::types::{AppCommand, ClipboardItem as CoreClipboardItem};
use crate::utils::paths;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::runtime::Runtime;

// Slint module include
slint::include_modules!();

struct SlintUi {
    window: slint::Weak<AppWindow>,
}

impl UiHandle for SlintUi {
    fn update_history(&self, items: Vec<CoreClipboardItem>) {
        let _ = self.window.upgrade_in_event_loop(move |window| {
            let model: Vec<ClipboardItem> = items.into_iter().map(|item| {
                ClipboardItem {
                    id: item.id.unwrap_or(0) as i32,
                    type_val: item.type_ as i32,
                    content: item.content.into(),
                    is_pinned: item.is_pinned,
                    source: item.source_app.unwrap_or_default().into(),
                }
            }).collect();

            // Slint VecModel
            let vec_model = std::rc::Rc::new(slint::VecModel::from(model));
            window.set_history_model(vec_model.into());
        });
    }

    fn update_search_results(&self, items: Vec<CoreClipboardItem>) {
        // Reuse update_history for now as they use the same view in MVP
        self.update_history(items);
    }

    fn hide_window(&self) {
        let _ = self.window.upgrade_in_event_loop(|window| {
            window.hide().unwrap();
        });
    }

    fn show_notification(&self, msg: String) {
        println!("Notification: {}", msg);
    }
}

fn main() -> Result<()> {
    env_logger::init();

    // 1. Initialize DB
    let db_path = paths::get_db_path();
    let db = Database::new(&db_path)?;

    // 2. Create Channels
    let (clipboard_tx, clipboard_rx) = mpsc::channel(100);
    let (command_tx, command_rx) = mpsc::channel(100);

    // 3. Initialize Slint Window
    let main_window = AppWindow::new()?;
    let window_handle = main_window.as_weak();

    // 4. Create UI Adapter
    let ui_handle = Arc::new(SlintUi {
        window: window_handle.clone(),
    });

    // 5. Start Core Manager in background thread (Tokio Runtime)
    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Start Clipboard Monitor
            let monitor = crate::core::clipboard::ClipboardMonitor::new(clipboard_tx)
                .expect("Failed to init clipboard monitor");

            tokio::spawn(monitor.run());

            // Start Manager
            let manager = Manager::new(db, clipboard_rx, command_rx, ui_handle);
            manager.run().await;
        });
    });

    // 6. Bind UI Callbacks
    let tx = command_tx.clone();
    main_window.on_search(move |query| {
        let _ = tx.blocking_send(AppCommand::Search(query.into()));
    });

    let tx = command_tx.clone();
    main_window.on_paste_item(move |id| {
        let _ = tx.blocking_send(AppCommand::PasteItem(id as i64));
    });

    let tx = command_tx.clone();
    main_window.on_delete_item(move |id| {
        let _ = tx.blocking_send(AppCommand::DeleteHistory(id as i64));
    });

    let tx = command_tx.clone();
    main_window.on_toggle_pin(move |id| {
        let _ = tx.blocking_send(AppCommand::TogglePin(id as i64));
    });

    let tx = command_tx.clone();
    main_window.on_toggle_queue_mode(move |enabled| {
        let _ = tx.blocking_send(AppCommand::ToggleQueueMode(enabled));
    });

    let tx = command_tx.clone();
    main_window.on_next_queue_item(move || {
        let _ = tx.blocking_send(AppCommand::NextQueueItem);
    });

    let tx = command_tx.clone();
    main_window.on_hide_window(move || {
        // Just hide
        // We can do it directly in UI or send command.
        // Sending command allows Manager to know state if needed.
        // For now, assume UI handles it or we call window.hide().
        // But here we are in a callback, so `main_window` is not accessible directly unless we clone weak.
        // Using command is better.
        // But AppCommand doesn't have Hide.
        // Let's just hide it via handle if we had one.
        // Or adding `Hide` to AppCommand.
    });

    // Global Hotkey setup (Optional/TODO)
    // global_hotkey::GlobalHotKeyManager...

    // 7. Run UI Loop
    main_window.run()?;

    Ok(())
}

pub mod core;
pub mod utils;
// ui depends on slint, so maybe keep it out of lib if we want to test lib without slint?
// But ui/window.rs is stubbed.
pub mod ui;

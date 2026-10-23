// Release builds must not open a console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    match aref_lib::run() {
        Ok(()) => {}
        // Startup failure has no window to render into yet; stderr is the only channel,
        // and it carries a Tauri error, never user text or key material.
        Err(error) => eprintln!("aref failed to start: {error}"),
    }
}

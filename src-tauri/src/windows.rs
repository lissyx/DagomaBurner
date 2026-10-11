//! The app window, ported from the Electron main process (appRunner.js).

use tauri::{App, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// `NODE_ENV=debug` opened a fullscreen, resizable window with the devtools.
fn debug_mode() -> bool {
    std::env::var("NODE_ENV").is_ok_and(|env| env == "debug")
}

/// Electron prevented `will-navigate`: the app only ever shows its own pages.
/// External links are handed to the system browser by the frontend.
fn is_app_url(url: &Url) -> bool {
    match url.scheme() {
        "tauri" | "about" | "data" | "blob" => true,
        // Windows serves the app from http(s)://tauri.localhost, `tauri dev` from the Vite server.
        "http" | "https" => matches!(url.host_str(), Some("tauri.localhost" | "localhost")),
        _ => false,
    }
}

pub fn open_main_window(app: &App) -> tauri::Result<()> {
    let debug = debug_mode();
    let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("DagomApp")
        .on_navigation(is_app_url)
        // Let the pages receive HTML5 file drops, as in Electron.
        .disable_drag_drop_handler();
    let builder = if debug {
        builder.fullscreen(true).resizable(true)
    } else {
        builder
            .inner_size(500.0, if cfg!(windows) { 480.0 } else { 450.0 })
            .resizable(false)
            .center()
    };
    let window = builder.build()?;

    #[cfg(debug_assertions)]
    if debug {
        window.open_devtools();
    }
    window.set_focus()
}

/// "Toggle Developer Tools" from the macOS menu; devtools only exist in debug builds.
#[tauri::command]
pub fn toggle_devtools(window: WebviewWindow) {
    #[cfg(debug_assertions)]
    if window.is_devtools_open() {
        window.close_devtools();
    } else {
        window.open_devtools();
    }
    #[cfg(not(debug_assertions))]
    let _ = window;
}

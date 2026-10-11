mod flasher;
mod serial;
mod windows;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(serial::SerialPorts::default())
        .setup(|app| Ok(windows::open_main_window(app)?))
        .invoke_handler(tauri::generate_handler![
            serial::serial_list,
            serial::serial_open,
            serial::serial_write,
            serial::serial_set,
            serial::serial_close,
            flasher::flash_firmware,
            windows::toggle_devtools,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DagomApp");
}

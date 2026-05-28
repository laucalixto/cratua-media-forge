mod commands;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_default_output_dir,
            check_output_overwrite,
            get_presets,
            create_preset,
            delete_preset,
            get_config,
            save_config,
            detect_ffmpeg,
            build_command_preview,
            start_encoding,
            cancel_encoding,
            get_history,
            clear_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

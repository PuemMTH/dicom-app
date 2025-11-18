// Modules
mod commands;
mod dicom_processor;
mod file_scanner;
mod models;

// Re-export commands for use in main
use commands::{greet, list_dicom_files, read_dicom_file, read_dicom_folder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ไม่ต้อง register! (เพราะเราใช้ features ใน Cargo.toml แล้ว)

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            read_dicom_file,
            read_dicom_folder,
            list_dicom_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
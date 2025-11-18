// Modules
mod commands;
mod dicom_processor;
mod file_scanner;
mod models;

// Re-export commands for use in main
use commands::{
    greet,
    list_dicom_files,
    read_dicom_file,
    read_dicom_folder,
    dicom_to_png,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ไม่ต้อง register! (เพราะเราใช้ features ใน Cargo.toml แล้ว)
    // dicom_to_png("/home/puemmth/project/test_data_dicom").unwrap();
    dicom_to_png("/mnt/data/project/test_data_dicom/DICOMtags_testdata1").unwrap();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            read_dicom_file,
            read_dicom_folder,
            list_dicom_files,
            dicom_to_png
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
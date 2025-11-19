// Modules
mod commands;
pub mod logic;
pub mod models;
pub mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ไม่ต้อง register! (เพราะเราใช้ features ใน Cargo.toml แล้ว)
    // Test conversion (uncomment to test):
    // dicom_to_png(
    //     "/mnt/data/project/test_data_dicom/DICOMtags_testdata1",
    //     "/tmp/output",
    // )
    // .unwrap();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::path::Path;

use crate::dicom_processor::{summarize_dicom, convert_dicom_to_png};
use crate::file_scanner::collect_dicom_file_paths;
use crate::models::{DicomFileDescriptor, DicomSummary};
use owo_colors::OwoColorize;

/// Command: อ่านและประมวลผลไฟล์ DICOM เดี่ยว
#[tauri::command]
pub async fn read_dicom_file(file_path: String) -> Result<DicomSummary, String> {
    // Run CPU-intensive work on a blocking thread pool
    // เพื่อไม่ให้บล็อก main thread และ UI
    tokio::task::spawn_blocking(move || summarize_dicom(Path::new(&file_path)))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

/// Command: อ่านและประมวลผลไฟล์ DICOM ทั้งโฟลเดอร์
#[tauri::command]
pub fn read_dicom_folder(folder_path: &str) -> Result<Vec<DicomSummary>, String> {
    let files = collect_dicom_file_paths(folder_path)?;
    
    if files.is_empty() {
        return Err("No DICOM (.dcm) files found in the selected folder.".into());
    }

    let mut summaries = Vec::with_capacity(files.len());
    for path in files {
        let file_label = path
            .file_name()
            .and_then(|v| v.to_str())
            .map(|name| name.to_string())
            .unwrap_or_else(|| path.display().to_string());

        match summarize_dicom(path.as_path()) {
            Ok(summary) => summaries.push(summary),
            Err(err) => summaries.push(DicomSummary::error(file_label, err)),
        }
    }

    Ok(summaries)
}

/// Command: แสดงรายการไฟล์ DICOM ในโฟลเดอร์ (ไม่ประมวลผล)
#[tauri::command]
pub fn list_dicom_files(folder_path: &str) -> Result<Vec<DicomFileDescriptor>, String> {
    let files = collect_dicom_file_paths(folder_path)?;
    let descriptors = files
        .into_iter()
        .map(|path| {
            let file_name = path
                .file_name()
                .and_then(|v| v.to_str())
                .map(|name| name.to_string())
                .unwrap_or_else(|| path.display().to_string());
            DicomFileDescriptor {
                file_name,
                file_path: path.to_string_lossy().into_owned(),
            }
        })
        .collect();

    Ok(descriptors)
}

/// Command: ทดสอบ (demo)
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn dicom_to_png(folder_path: &str) -> Result<(), String> {
    let files = collect_dicom_file_paths(folder_path)?;
    for dicom_file in files {
        let folder_path = dicom_file.parent().ok_or("Failed to get parent folder".to_string())?;
        match convert_dicom_to_png(&dicom_file, folder_path) {
            Ok(summary) => {
                print!("Conversion successful for file: {}\n", summary.file_name.green());
                print!(
                    "Image dimensions: {}x{}, Bits Allocated: {}\n",
                    summary.rows.unwrap_or(0),
                    summary.columns.unwrap_or(0),
                    summary.bits_allocated.unwrap_or(0)
                );
            },
            Err(err) => {
                print!("Conversion failed for file: {}\n", dicom_file.display().red());
                print!("Error: {}\n", err.red());
            }
        }
    }
    Ok(())


}

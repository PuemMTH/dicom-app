use std::path::Path;

use crate::dicom_processor::{summarize_dicom, convert_dicom_to_png};
use crate::file_scanner::collect_dicom_file_paths;
use crate::models::{DicomFileDescriptor, DicomSummary, ConversionResult, FileDetail};
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
pub fn dicom_to_png(input_folder: &str, output_folder: &str) -> Result<ConversionResult, String> {
    use std::fs;

    // สร้าง main output folder ตามรูปแบบ Python: output_{input_folder_name}
    let input_path = Path::new(input_folder);
    let input_folder_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Invalid input folder name")?;

    let main_output_folder = Path::new(output_folder)
        .join(format!("output_{}", input_folder_name));

    // สร้าง png_file subfolder
    let png_output_folder = main_output_folder.join("png_file");
    fs::create_dir_all(&png_output_folder)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // รวบรวมไฟล์ DICOM ทั้งหมด
    let files = collect_dicom_file_paths(input_folder)?;
    let total_files = files.len();

    print!("Converting {} DICOM files...\n", total_files);

    let mut successful = 0;
    let mut failed = 0;
    let mut failed_files = Vec::new();
    let mut error_details = Vec::new();
    let mut all_file_details = Vec::new();
    let mut metadata_list = Vec::new(); // Python: data = []

    // แปลงไฟล์ทีละไฟล์
    for (index, dicom_file) in files.iter().enumerate() {
        let file_name = dicom_file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_path_str = dicom_file.to_string_lossy().to_string();

        // คำนวณ relative path จาก input folder เพื่อเก็บโครงสร้างโฟลเดอร์
        let relative_path = dicom_file
            .strip_prefix(input_path)
            .ok()
            .and_then(|p| p.parent())
            .unwrap_or(Path::new(""));

        // สร้าง output path ที่เก็บโครงสร้างโฟลเดอร์ไว้
        let output_subfolder = png_output_folder.join(relative_path);

        // แสดง progress ทุก 10 ไฟล์ หรือไฟล์สุดท้าย
        if (index + 1) % 1 == 0 || index + 1 == total_files {
            print!("Progress: {}/{}\n", index + 1, total_files);
        }

        match convert_dicom_to_png(&dicom_file, &output_subfolder) {
            Ok((_summary, metadata)) => {
                // Python: data.append(metadata)
                metadata_list.push(metadata);

                successful += 1;
                all_file_details.push(FileDetail::success(
                    file_name,
                    file_path_str,
                    "PNG".to_string(),
                ));
            },
            Err(err) => {
                // แสดง error เฉพาะไฟล์ที่ล้มเหลว
                print!("✗ Failed: {} - {}\n", file_name.red(), err.red());

                failed += 1;
                failed_files.push(file_name.clone());

                let detail = FileDetail::error(
                    file_name,
                    file_path_str,
                    "ConversionError".to_string(),
                    err,
                    "PNG".to_string(),
                );

                error_details.push(detail.clone());
                all_file_details.push(detail);
            }
        }
    }

    // Summary
    print!("\n{}\n", "=".repeat(50));
    print!("✓ Conversion completed\n");
    print!("  Total:      {}\n", total_files);
    print!("  Successful: {}\n", successful.to_string().green());
    if failed > 0 {
        print!("  Failed:     {}\n", failed.to_string().red());
    }
    print!("  Output:     {}\n", main_output_folder.display());
    print!("{}\n", "=".repeat(50));

    Ok(ConversionResult {
        main_output_folder: main_output_folder.to_string_lossy().to_string(),
        total: total_files,
        successful,
        failed,
        failed_files,
        error_details,
        all_file_details,
        metadata: metadata_list,
    })
}

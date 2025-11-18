use std::fs;
use std::path::{Path, PathBuf};

/// สแกนหาไฟล์ DICOM แบบ recursive ในโฟลเดอร์
fn collect_dicom_file_paths_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let dir_entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

    for entry in dir_entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| e.to_string())?;

        if file_type.is_dir() {
            // เข้าไปใน subdirectory
            collect_dicom_file_paths_recursive(&path, files)?;
        } else if file_type.is_file() {
            // เช็คว่ามี extension .dcm หรือไม่
            let has_dcm_ext = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("dcm"))
                .unwrap_or(false);

            if has_dcm_ext {
                files.push(path);
            }
        }
    }

    Ok(())
}

/// รวบรวม path ของไฟล์ DICOM ทั้งหมดในโฟลเดอร์ (รวม subfolders)
pub fn collect_dicom_file_paths(folder_path: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let root_path = Path::new(folder_path);

    // ตรวจสอบว่า path มีอยู่จริง
    if !root_path.exists() {
        return Err(format!("Path does not exist: {}", folder_path));
    }

    // ตรวจสอบว่าเป็น directory
    if !root_path.is_dir() {
        return Err(format!("Path is not a directory: {}", folder_path));
    }

    // สแกนหาไฟล์
    collect_dicom_file_paths_recursive(root_path, &mut files)?;
    Ok(files)
}

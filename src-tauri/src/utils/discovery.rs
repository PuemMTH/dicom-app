use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn collect_dicom_files(input_folder: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(input_folder).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let path = entry.path().to_path_buf();
            if looks_like_dicom(&path) {
                files.push(path);
            }
        }
    }
    files
}

fn looks_like_dicom(path: &Path) -> bool {
    const MAGIC_OFFSET: usize = 128;
    const BUFFER_SIZE: usize = MAGIC_OFFSET + 4;

    let extension_matches = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "dcm" | "dicom" | "ima"))
        .unwrap_or(false);

    if extension_matches {
        return true;
    }

    if let Ok(mut file) = File::open(path) {
        let mut buffer = [0u8; BUFFER_SIZE];
        if file.read_exact(&mut buffer).is_ok() {
            return &buffer[MAGIC_OFFSET..] == b"DICM";
        }
    }
    false
}

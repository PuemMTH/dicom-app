use std::path::Path;

use dicom_object::open_file;
use dicom_pixeldata::PixelDecoder;
use dicom_transfer_syntax_registry::{TransferSyntaxIndex, TransferSyntaxRegistry};

use crate::models::DicomSummary;

/// ประมวลผลไฟล์ DICOM และแยกข้อมูลสำคัญออกมา
pub fn summarize_dicom(path: &Path) -> Result<DicomSummary, String> {
    // เปิดไฟล์ DICOM
    let obj = open_file(path).map_err(|e| e.to_string())?;
    println!("File opened: {}", path.display());

    // ดึง Transfer Syntax
    let ts_uid = obj.meta().transfer_syntax();
    let ts_name = TransferSyntaxRegistry::default()
        .get(ts_uid)
        .map(|ts| ts.name())
        .unwrap_or(ts_uid);
    println!("Transfer Syntax: {}", ts_name);

    // Decode pixel data
    let pixel_data = obj.decode_pixel_data().map_err(|e| e.to_string())?;
    println!("Pixel data decoded!");
    println!("Rows: {}", pixel_data.rows());
    println!("Cols: {}", pixel_data.columns());
    println!("Bits Allocated: {}", pixel_data.bits_allocated());

    // ดึงชื่อไฟล์
    let file_label = path
        .file_name()
        .and_then(|v| v.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string());

    // สร้าง summary
    Ok(DicomSummary::success(
        file_label,
        pixel_data.rows(),
        pixel_data.columns(),
        pixel_data.bits_allocated(),
        ts_name.to_string(),
    ))
}

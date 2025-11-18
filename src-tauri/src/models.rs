use serde::Serialize;

/// ข้อมูลสรุปของไฟล์ DICOM
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DicomSummary {
    pub file_name: String,
    pub rows: Option<u32>,
    pub columns: Option<u32>,
    pub bits_allocated: Option<u16>,
    pub transfer_syntax: Option<String>,
    pub status: String,
    pub message: Option<String>,
}

impl DicomSummary {
    /// สร้าง DicomSummary สำหรับกรณีสำเร็จ
    pub fn success(
        file_name: String,
        rows: u32,
        columns: u32,
        bits_allocated: u16,
        transfer_syntax: String,
    ) -> Self {
        Self {
            file_name,
            rows: Some(rows),
            columns: Some(columns),
            bits_allocated: Some(bits_allocated),
            transfer_syntax: Some(transfer_syntax),
            status: "ok".to_string(),
            message: Some(format!(
                "Rows: {}, Cols: {}, Bits: {}",
                rows, columns, bits_allocated
            )),
        }
    }

    /// สร้าง DicomSummary สำหรับกรณีเกิด error
    pub fn error(file_name: String, err: String) -> Self {
        Self {
            file_name,
            rows: None,
            columns: None,
            bits_allocated: None,
            transfer_syntax: None,
            status: "error".to_string(),
            message: Some(err),
        }
    }
}

/// ข้อมูลพื้นฐานของไฟล์ DICOM (ชื่อและ path)
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DicomFileDescriptor {
    pub file_name: String,
    pub file_path: String,
}

/// รายละเอียดไฟล์ที่แปลง (สำเร็จหรือล้มเหลว)
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileDetail {
    pub file_name: String,
    pub file_path: String,
    pub success: bool,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub conversion_type: String,
}

impl FileDetail {
    pub fn success(file_name: String, file_path: String, conversion_type: String) -> Self {
        Self {
            file_name,
            file_path,
            success: true,
            error_type: None,
            error_message: None,
            conversion_type,
        }
    }

    pub fn error(file_name: String, file_path: String, error_type: String, error_message: String, conversion_type: String) -> Self {
        Self {
            file_name,
            file_path,
            success: false,
            error_type: Some(error_type),
            error_message: Some(error_message),
            conversion_type,
        }
    }
}

/// ผลลัพธ์การแปลงไฟล์
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub main_output_folder: String,
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub failed_files: Vec<String>,
    pub error_details: Vec<FileDetail>,
    pub all_file_details: Vec<FileDetail>,
    pub metadata: Vec<DicomMetadata>, // เพิ่ม metadata array
}

/// DICOM Metadata ตามรูปแบบ Python
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DicomMetadata {
    pub f_name: String,
    pub study_date: Option<String>,
    pub modality: Option<String>,
    pub manufacturer: Option<String>,
    pub study_description: Option<String>,
    pub series_description: Option<String>,
    pub institution_name: Option<String>,
    pub im_width: Option<u32>,
    pub im_height: Option<u32>,
    pub pixel_spacing: Option<Vec<f64>>,
}

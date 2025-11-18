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

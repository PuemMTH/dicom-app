use chrono::NaiveDate;
use dicom::core::Tag;
use dicom_object::DefaultDicomObject;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FileMetadata {
    pub folder_relative: PathBuf,
    pub file_name: String,
    pub study_date: Option<String>,
    pub modality: Option<String>,
    pub manufacturer: Option<String>,
    pub study_description: Option<String>,
    pub series_description: Option<String>,
    pub institution_name: Option<String>,
    pub im_width: Option<u32>,
    pub im_height: Option<u32>,
    pub pixel_spacing: Option<String>,
}

pub fn dicom_text(obj: &DefaultDicomObject, tag: Tag) -> Option<String> {
    obj.element(tag)
        .ok()
        .and_then(|element| element.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn dicom_date(obj: &DefaultDicomObject, tag: Tag) -> Option<String> {
    let raw = dicom_text(obj, tag)?;
    let cleaned: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if cleaned.len() < 8 {
        return None;
    }
    NaiveDate::parse_from_str(&cleaned[..8], "%Y%m%d")
        .ok()
        .map(|date| date.format("%Y-%m-%d").to_string())
}

pub fn pixel_spacing(obj: &DefaultDicomObject) -> Option<String> {
    dicom_text(obj, Tag(0x0028, 0x0030)).map(|raw| raw.replace('\\', ", "))
}

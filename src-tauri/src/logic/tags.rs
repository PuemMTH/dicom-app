use anyhow::{Context, Result};
use dicom::core::dictionary::DataDictionary;
use dicom::core::header::Header;
use dicom::object::open_file;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct DicomTag {
    pub group: u16,
    pub element: u16,
    pub name: String,
    pub vr: String,
    pub value: String,
}

pub fn read_all_tags(path: &Path) -> Result<Vec<DicomTag>> {
    let obj =
        open_file(path).with_context(|| format!("Failed to open DICOM file {}", path.display()))?;
    let mut tags = Vec::new();

    for element in obj.into_iter() {
        let tag = element.tag();
        let name = dicom::dictionary_std::StandardDataDictionary
            .by_tag(tag)
            .map(|e| e.alias.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let value = if let Ok(v) = element.to_str() {
            // if tag PixelData then skip 7FE0,0010
            if (tag.0, tag.1) == (0x7fe0, 0x0010) {
                "<binary data>".to_string()
            } else {
                v.to_string()
            }
        } else {
            "<binary data>".to_string()
        };

        tags.push(DicomTag {
            group: tag.0,
            element: tag.1,
            name,
            vr: element.vr().to_string().to_string(),
            value,
        });
    }

    Ok(tags)
}

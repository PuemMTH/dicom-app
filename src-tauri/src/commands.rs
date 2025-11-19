use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn convert_dicom(
    app: AppHandle,
    input: String,
    output: String,
    skip_excel: bool,
    flatten_output: bool,
) -> Result<crate::logic::workflow::ConversionReport, String> {
    use crate::logic::workflow::convert_dicom_to_png;
    match convert_dicom_to_png(
        std::path::Path::new(&input),
        std::path::Path::new(&output),
        !skip_excel,
        flatten_output,
        |progress| {
            let _ = app.emit("conversion_progress", progress);
        },
    ) {
        Ok(report) => {
            // Open the output folder after conversion completes
            let _ = app.opener().open_url(
                report.output_folder.to_string_lossy().as_ref(),
                None::<&str>,
            );
            Ok(report)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn anonymize_dicom(
    app: AppHandle,
    input: String,
    output: String,
    tags: Vec<(u16, u16)>,
    replacement: String,
) -> Result<crate::logic::anonymize::AnonymizationReport, String> {
    use crate::logic::anonymize::anonymize_dicom;
    match anonymize_dicom(
        std::path::Path::new(&input),
        std::path::Path::new(&output),
        tags,
        replacement,
        |progress| {
            let _ = app.emit("anonymization_progress", progress);
        },
    ) {
        Ok(report) => {
            // Open the output folder after anonymization completes
            let _ = app.opener().open_url(
                report.output_folder.to_string_lossy().as_ref(),
                None::<&str>,
            );
            Ok(report)
        }
        Err(e) => Err(e.to_string()),
    }
}

use serde::Deserialize;

#[derive(Deserialize)]
pub struct DicomToPngInput {
    pub input: String,
    pub output: String,
    pub skip_excel: bool,
    pub flatten_output: bool,
}

#[derive(Deserialize)]
pub struct AnonymizeDicomInput {
    pub input: String,
    pub output: String,
    pub tags: Vec<(u16, u16)>,
    pub replacement: String,
}

#[derive(Deserialize, Default)]
pub struct DicomProcessInput {
    pub convert: Option<DicomToPngInput>,
    pub anonymize: Option<AnonymizeDicomInput>,
}

#[tauri::command]
pub async fn process_dicom(
    app: AppHandle,
    input: DicomProcessInput,
) -> Result<ProcessReport, String> {
    use crate::logic::anonymize::anonymize_dicom as do_anonymize;
    use crate::logic::workflow::convert_dicom_to_png;

    let mut report = ProcessReport::default();

    // Process conversion if requested
    if let Some(convert_input) = input.convert {
        match convert_dicom_to_png(
            std::path::Path::new(&convert_input.input),
            std::path::Path::new(&convert_input.output),
            !convert_input.skip_excel,
            convert_input.flatten_output,
            |progress| {
                let _ = app.emit("conversion_progress", progress);
            },
        ) {
            Ok(conversion_report) => {
                let _ = app.opener().open_url(
                    conversion_report.output_folder.to_string_lossy().as_ref(),
                    None::<&str>,
                );
                report.conversion = Some(conversion_report);
            }
            Err(e) => return Err(format!("Conversion failed: {}", e)),
        }
    }

    // Process anonymization if requested
    if let Some(anonymize_input) = input.anonymize {
        match do_anonymize(
            std::path::Path::new(&anonymize_input.input),
            std::path::Path::new(&anonymize_input.output),
            anonymize_input.tags,
            anonymize_input.replacement,
            |progress| {
                let _ = app.emit("anonymization_progress", progress);
            },
        ) {
            Ok(anonymization_report) => {
                let _ = app.opener().open_url(
                    anonymization_report
                        .output_folder
                        .to_string_lossy()
                        .as_ref(),
                    None::<&str>,
                );
                report.anonymization = Some(anonymization_report);
            }
            Err(e) => return Err(format!("Anonymization failed: {}", e)),
        }
    }

    Ok(report)
}

#[derive(serde::Serialize, Default)]
pub struct ProcessReport {
    pub conversion: Option<crate::logic::workflow::ConversionReport>,
    pub anonymization: Option<crate::logic::anonymize::AnonymizationReport>,
}

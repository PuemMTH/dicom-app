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

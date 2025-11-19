use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn convert_dicom(app: AppHandle, input: String, output: String, skip_excel: bool) {
    use crate::logic::workflow::convert_dicom_to_png;
    if let Ok(report) = convert_dicom_to_png(
        std::path::Path::new(&input),
        std::path::Path::new(&output),
        !skip_excel,
        |progress| {
            let _ = app.emit("conversion_progress", progress);
        },
    ) {
        // Open the output folder after conversion completes
        let _ = app.opener().open_url(
            report.output_folder.to_string_lossy().as_ref(),
            None::<&str>,
        );
    }
}

#[tauri::command]
pub async fn anonymize_dicom(
    app: AppHandle,
    input: String,
    output: String,
    tags: Vec<(u16, u16)>,
    replacement: String,
) {
    use crate::logic::anonymize::anonymize_dicom;
    if let Ok(report) = anonymize_dicom(
        std::path::Path::new(&input),
        std::path::Path::new(&output),
        tags,
        replacement,
        |progress| {
            let _ = app.emit("anonymization_progress", progress);
        },
    ) {
        // Open the output folder after anonymization completes
        let _ = app.opener().open_url(
            report.output_folder.to_string_lossy().as_ref(),
            None::<&str>,
        );
    }
}

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

#[derive(Clone, serde::Serialize)]
pub struct LogEntry {
    pub file_name: String,
    pub file_path: String,
    pub success: bool,
    pub status: String,
    pub message: String,
    pub conversion_type: String,
}

pub struct LogWriter {
    file: std::fs::File,
}

impl LogWriter {
    pub fn new(output_folder: &Path) -> Result<Self> {
        let log_path = output_folder.join("logs.csv");
        let file_exists = log_path.exists();

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("Failed to open log file: {}", log_path.display()))?;

        if !file_exists {
            writeln!(file, "file_name,file_path,status,conversion_type")?;
        }

        Ok(Self { file })
    }

    pub fn write_entry(&mut self, log: &LogEntry) -> Result<()> {
        writeln!(
            self.file,
            "{},{},{},{}",
            escape_csv(&log.file_name),
            escape_csv(&log.file_path),
            escape_csv(&log.status),
            escape_csv(&log.conversion_type)
        )?;
        self.file.flush()?;
        Ok(())
    }
}

pub fn write_logs(output_folder: &Path, logs: &[LogEntry]) -> Result<()> {
    if logs.is_empty() {
        return Ok(());
    }
    let mut writer = LogWriter::new(output_folder)?;
    for log in logs {
        writer.write_entry(log)?;
    }
    Ok(())
}

fn escape_csv(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

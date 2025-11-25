use crate::utils::discovery::collect_dicom_files;
use anyhow::Result;
use dicom::core::dictionary::DataDictionary;
use dicom::core::Tag;
use dicom::object::open_file;
use dicom_pixeldata::PixelDecoder;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Serialize, Clone)]
pub struct TagStat {
    pub group: u16,
    pub element: u16,
    pub name: String,
    pub value_counts: HashMap<String, usize>,
}

pub struct StatsCache(pub std::sync::Mutex<HashMap<(String, Vec<(u16, u16)>), Vec<TagStat>>>);

impl Default for StatsCache {
    fn default() -> Self {
        Self(std::sync::Mutex::new(HashMap::new()))
    }
}

#[derive(Clone, Serialize)]
pub struct StatsProgress {
    pub current: usize,
    pub total: usize,
}

pub fn calculate_stats<F>(
    folder: &Path,
    tags: Vec<(u16, u16)>,
    progress_callback: F,
) -> Result<Vec<TagStat>>
where
    F: Fn(StatsProgress) + Sync + Send,
{
    let files = collect_dicom_files(folder);
    let total = files.len();
    let processed_count = AtomicUsize::new(0);

    // Map to store aggregated counts: (group, element) -> HashMap<Value, Count>
    // We use a Mutex to allow safe concurrent updates, or we can reduce.
    // Reducing is better for performance to avoid lock contention.

    let stats_map: HashMap<(u16, u16), HashMap<String, usize>> = files
        .par_iter()
        .fold(
            || HashMap::new(),
            |mut acc: HashMap<(u16, u16), HashMap<String, usize>>, file_path| {
                let current = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                if current % 10 == 0 || current == total {
                    progress_callback(StatsProgress { current, total });
                }

                if let Ok(obj) = open_file(file_path) {
                    for &(group, element) in &tags {
                        let tag = Tag(group, element);

                        let value = if (group, element) == (0x7fe0, 0x0010) {
                            if obj.element(tag).is_err() {
                                "Missing".to_string()
                            } else {
                                match obj.decode_pixel_data() {
                                    Ok(data) => match data.to_dynamic_image(0) {
                                        Ok(_) => "Binary".to_string(),
                                        Err(_) => "Error".to_string(),
                                    },
                                    Err(_) => "Error".to_string(),
                                }
                            }
                        } else if let Ok(elem) = obj.element(tag) {
                            if let Ok(v) = elem.to_str() {
                                v.to_string()
                            } else {
                                "<binary/unknown>".to_string()
                            }
                        } else {
                            "Missing".to_string()
                        };

                        acc.entry((group, element))
                            .or_default()
                            .entry(value)
                            .and_modify(|c| *c += 1)
                            .or_insert(1);
                    }
                }

                acc
            },
        )
        .reduce(
            || HashMap::new(),
            |mut acc, part| {
                for (tag_key, counts) in part {
                    let entry = acc.entry(tag_key).or_default();
                    for (val, count) in counts {
                        *entry.entry(val).or_default() += count;
                    }
                }
                acc
            },
        );

    // Convert to result vector
    let mut result = Vec::new();
    for (group, element) in tags {
        if let Some(counts) = stats_map.get(&(group, element)) {
            let name = dicom::dictionary_std::StandardDataDictionary
                .by_tag(Tag(group, element))
                .map(|e| e.alias.to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            result.push(TagStat {
                group,
                element,
                name,
                value_counts: counts.clone(),
            });
        }
    }

    Ok(result)
}

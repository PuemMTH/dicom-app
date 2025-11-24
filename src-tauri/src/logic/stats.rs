use crate::utils::discovery::collect_dicom_files;
use anyhow::Result;
use dicom::core::dictionary::DataDictionary;
use dicom::core::Tag;
use dicom::object::open_file;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct TagStat {
    pub group: u16,
    pub element: u16,
    pub name: String,
    pub value_counts: HashMap<String, usize>,
}

pub fn calculate_stats(folder: &Path, tags: Vec<(u16, u16)>) -> Result<Vec<TagStat>> {
    let files = collect_dicom_files(folder);

    // Map to store aggregated counts: (group, element) -> HashMap<Value, Count>
    // We use a Mutex to allow safe concurrent updates, or we can reduce.
    // Reducing is better for performance to avoid lock contention.

    let stats_map: HashMap<(u16, u16), HashMap<String, usize>> = files
        .par_iter()
        .fold(
            || HashMap::new(),
            |mut acc: HashMap<(u16, u16), HashMap<String, usize>>, file_path| {
                if let Ok(obj) = open_file(file_path) {
                    for &(group, element) in &tags {
                        let tag = Tag(group, element);
                        let value = if let Ok(elem) = obj.element(tag) {
                            if let Ok(v) = elem.to_str() {
                                v.to_string()
                            } else {
                                "<binary/unknown>".to_string()
                            }
                        } else {
                            "<missing>".to_string()
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

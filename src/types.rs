use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PboInfo {
    pub hash: String,
    pub failed: bool,
    pub skip_reason: Option<SkipReason>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SkipReason {
    Empty,
    NoMatchingFiles,
    InvalidFormat,
    Failed,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ScanDatabase {
    pbos: HashMap<String, PboInfo>,
}

#[derive(Debug)]
pub struct PboScanResult {
    pub path: PathBuf,
    pub hash: String,
    pub expected_files: Vec<String>,
}

impl ScanDatabase {
    pub fn load_or_create(path: &Path) -> Result<Self> {
        if path.exists() {
            let file = std::fs::File::open(path)?;
            Ok(serde_json::from_reader(file)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    pub fn get_pbo_info(&self, path: &Path) -> Option<&PboInfo> {
        self.pbos.get(&path.to_string_lossy().to_string())
    }

    pub fn update_pbo(&mut self, path: &Path, hash: &str, failed: bool) {
        self.pbos.insert(
            path.to_string_lossy().to_string(),
            PboInfo {
                hash: hash.to_string(),
                failed,
                skip_reason: None,
            },
        );
    }

    pub fn update_pbo_with_reason(&mut self, path: &Path, hash: &str, failed: bool, reason: SkipReason) {
        self.pbos.insert(
            path.to_string_lossy().to_string(),
            PboInfo {
                hash: hash.to_string(),
                failed,
                skip_reason: Some(reason),
            },
        );
    }

    pub fn get_stats(&self) -> ScanStats {
        let mut stats = ScanStats {
            total: self.pbos.len(),
            ..Default::default()
        };

        for info in self.pbos.values() {
            if info.failed {
                match info.skip_reason {
                    Some(SkipReason::Empty) => stats.empty += 1,
                    Some(SkipReason::NoMatchingFiles) => stats.no_matching_files += 1,
                    Some(SkipReason::InvalidFormat) => stats.invalid_format += 1,
                    Some(SkipReason::Failed) => stats.failed += 1,
                    None => stats.failed += 1,
                }
                stats.previously_failed += 1;
            } else {
                stats.processed += 1;
            }
        }

        stats
    }
}

#[derive(Debug, Default)]
pub struct ScanStats {
    pub total: usize,
    pub processed: usize,
    pub empty: usize,
    pub no_matching_files: usize,
    pub invalid_format: usize,
    pub failed: usize,
    pub unchanged: usize,
    pub previously_failed: usize,
} 
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct PboInfo {
    pub hash: String,
    pub failed: bool,
    pub skip_reason: Option<SkipReason>,
    pub expected_files: Option<Vec<String>>,
    pub extracted_files: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SkipReason {
    Empty,
    NoMatchingFiles,
    InvalidFormat,
    Failed,
    MissingExpectedFiles,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ScanDatabase {
    pub pbos: HashMap<String, PboInfo>,
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
    pub missing_expected_files: usize,
} 
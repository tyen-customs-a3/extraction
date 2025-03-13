use std::path::PathBuf;

#[derive(Debug)]
pub struct PboScanResult {
    pub path: PathBuf,
    pub expected_files: Vec<String>,
}
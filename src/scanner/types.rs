use std::path::PathBuf;

#[derive(Debug)]
pub struct PboHashResult {
    pub path: PathBuf,
    pub hash: String,
}

#[derive(Debug)]
pub struct PboScanResult {
    pub path: PathBuf,
    pub hash: String,
    pub expected_files: Vec<String>,
} 
use std::sync::{Arc, Mutex};
use std::path::Path;
use anyhow::Result;
use futures::stream::{self, StreamExt};
use indicatif::ProgressBar;
use log::debug;
use walkdir::WalkDir;

use super::types::{PboScanResult, PboHashResult};
use super::utils;
use crate::extraction::database::ScanDatabase;

pub struct PreScanner<'a> {
    input_dir: &'a Path,
    extensions: &'a str,
    db: Arc<Mutex<ScanDatabase>>,
    threads: usize,
    timeout: u32,
}

impl<'a> PreScanner<'a> {
    pub fn new(
        input_dir: &'a Path,
        extensions: &'a str,
        db: Arc<Mutex<ScanDatabase>>,
        threads: usize,
        timeout: u32,
    ) -> Self {
        Self {
            input_dir,
            extensions,
            db,
            threads,
            timeout,
        }
    }

    pub async fn scan_all(&self, progress: ProgressBar) -> Result<Vec<PboHashResult>> {
        // Find all PBO files in the input directory
        debug!("Finding all PBO files in {}", self.input_dir.display());
        let pbo_paths: Vec<_> = WalkDir::new(self.input_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension()
                    .map(|ext| ext == "pbo")
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_owned())
            .collect();

        debug!("Found {} PBO files", pbo_paths.len());
        progress.set_length(pbo_paths.len() as u64);

        // Process PBOs in chunks to limit concurrency
        let mut results = Vec::new();
        let chunks = stream::iter(pbo_paths)
            .chunks(self.threads);
            
        let mut stream = chunks.map(|chunk| {
            let db = Arc::clone(&self.db);
            let chunk_size = chunk.len();
            
            tokio::spawn(async move {
                let mut chunk_results = Vec::new();
                for path in chunk {
                    debug!("Checking PBO hash in thread: {}", path.display());
                    if let Ok(result) = utils::check_pbo_hash(&path, &db) {
                        chunk_results.push(result);
                    }
                }
                (chunk_results, chunk_size)
            })
        });

        // Collect results from all threads
        while let Some(chunk_handle) = stream.next().await {
            if let Ok((chunk_results, chunk_size)) = chunk_handle.await {
                debug!("Thread completed processing {} PBOs", chunk_size);
                results.extend(chunk_results);
                progress.inc(chunk_size as u64);
            }
        }

        progress.finish_with_message("Hash check complete");
        debug!("Hash check complete, found {} PBOs needing processing", results.len());
        Ok(results)
    }

    // Simplified function that leverages our common utility
    pub fn scan_pbo(
        path: &Path,
        extensions: &str,
        db: &Arc<Mutex<ScanDatabase>>,
        timeout: u32,
    ) -> Result<PboScanResult> {
        // First check if we need to process this PBO
        let hash_result = utils::check_pbo_hash(path, db)?;
        
        // Then scan for matching files
        utils::scan_pbo_contents(path, &hash_result.hash, extensions, timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use crate::extraction::utils;

    fn create_test_pbo(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    #[tokio::test]
    async fn test_prescanner_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db = Arc::new(Mutex::new(ScanDatabase::default()));
        let scanner = PreScanner::new(
            temp_dir.path(),
            "txt,cpp",
            db,
            4,
            30,
        );

        let progress = ProgressBar::new(0);
        let results = scanner.scan_all(progress).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_prescanner_with_pbos() {
        let temp_dir = TempDir::new().unwrap();
        let db = Arc::new(Mutex::new(ScanDatabase::default()));
        
        // Create test PBO files with mock content
        let content = b"PboPrefix=test\nVersion=1.0\nFile1.txt=123\nFile2.cpp=456\n";
        create_test_pbo(temp_dir.path(), "test1.pbo", content);
        create_test_pbo(temp_dir.path(), "test2.pbo", content);
        
        let scanner = PreScanner::new(
            temp_dir.path(),
            "txt,cpp",
            db,
            4,
            30,
        );

        let progress = ProgressBar::new(0);
        let results = scanner.scan_all(progress).await.unwrap();
        
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.path.file_name().unwrap() == "test1.pbo"));
        assert!(results.iter().any(|r| r.path.file_name().unwrap() == "test2.pbo"));
    }

    #[test]
    fn test_scan_pbo_unchanged() {
        let temp_dir = TempDir::new().unwrap();
        let db = Arc::new(Mutex::new(ScanDatabase::default()));
        
        // Create a test PBO with mock content
        let content = b"PboPrefix=test\nVersion=1.0\nFile1.txt=123\nFile2.cpp=456\n";
        let pbo_path = create_test_pbo(temp_dir.path(), "unchanged.pbo", content);
        
        // First scan should succeed
        let result = PreScanner::scan_pbo(&pbo_path, "txt,cpp", &db, 30);
        assert!(result.is_ok());
        
        // Update database to mark it as processed
        let hash = utils::calculate_file_hash(&pbo_path).unwrap();
        {
            let mut db = db.lock().unwrap();
            db.update_pbo(&pbo_path, &hash, false);
        }
        
        // Second scan should return unchanged error
        let result = PreScanner::scan_pbo(&pbo_path, "txt,cpp", &db, 30);
        assert!(result.is_err());
    }
}
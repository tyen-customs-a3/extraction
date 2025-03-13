#[allow(dead_code)]
use std::path::Path;
use log::{debug, trace, warn};
use walkdir::WalkDir;
use anyhow::Result;
use rayon::prelude::*;

use super::processor::PboProcessor;
use super::utils;

pub struct ScanCoordinator<'a> {
    input_dir: &'a Path,
    cache_dir: &'a Path,
    extensions: &'a str,
    threads: usize,
    timeout: u32
}

impl<'a> ScanCoordinator<'a> {
    pub fn new(
        input_dir: &'a Path,
        cache_dir: &'a Path,
        extensions: &'a str,
        threads: usize,
        timeout: u32,
    ) -> Result<Self> {
        Ok(Self {
            input_dir,
            cache_dir,
            extensions,
            threads,
            timeout,
        })
    }

    pub async fn run(&self) -> Result<()> {
        debug!("Starting extraction process with the following configuration:");
        debug!("  Input directory: {}", self.input_dir.display());
        debug!("  Cache directory: {}", self.cache_dir.display());
        debug!("  Extensions filter: {}", self.extensions);
        debug!("  Threads: {}", self.threads);
        debug!("  Timeout: {} seconds", self.timeout);

        // Verify directories exist
        if !self.input_dir.exists() {
            return Err(anyhow::anyhow!("Input directory does not exist: {}", self.input_dir.display()));
        }

        // Create cache directory if it doesn't exist
        if !self.cache_dir.exists() {
            debug!("Creating cache directory: {}", self.cache_dir.display());
            std::fs::create_dir_all(self.cache_dir)?;
        }

        // Count total PBOs first for reference
        debug!("Scanning input directory for PBO files...");
        let total_pbo_files = WalkDir::new(self.input_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension()
                    .map(|ext| ext == "pbo")
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        let total_pbo_count = total_pbo_files.len();

        if total_pbo_count == 0 {
            return Err(anyhow::anyhow!("No PBO files found in input directory: {}", self.input_dir.display()));
        }

        debug!("Found {} PBO files to process", total_pbo_count);

        // Initialize processor with multithreading
        debug!("Initializing PBO processor for extraction with {} threads", self.threads);
        let processor = PboProcessor::new(
            self.input_dir,
            self.cache_dir,
            self.extensions,
            self.threads,
            self.timeout,
        );

        // Process PBOs in parallel
        let scan_results: Vec<_> = total_pbo_files
            .par_iter()
            .map(|entry| {
                utils::scan_pbo_contents(entry.path(), self.extensions, self.timeout)
            })
            .filter_map(|result| {
                match result {
                    Ok(result) => {
                        debug!("Found {} matching files in {}", result.expected_files.len(), result.path.display());
                        if !result.expected_files.is_empty() {
                            trace!("Files to extract from {}: {:?}", result.path.display(), result.expected_files);
                        }
                        Some(result)
                    },
                    Err(e) => {
                        warn!("Failed to process PBO: {}", e);
                        None
                    }
                }
            })
            .collect();

        debug!("PBO scan complete:");
        debug!("  Total PBOs scanned: {}", scan_results.len());

        // Process PBOs for extraction
        debug!("Starting extraction from {} PBOs", scan_results.len());
        processor.process_all(&scan_results)?;

        Ok(())
    }
}
#[allow(dead_code)]
use std::path::Path;
use anyhow::Result;
use log::{debug, trace, warn};
use pbo_tools::{
    core::api::{PboApi, PboApiOps},
    extract::{ExtractOptions, ExtractResult},
    core::config::PboConfig,
};
use rayon::prelude::*;

use super::types::PboScanResult;

pub struct PboProcessor<'a> {
    input_dir: &'a Path,
    cache_dir: &'a Path,
    extensions: &'a str,
    threads: usize,
    timeout: u32,
}

impl<'a> PboProcessor<'a> {
    pub fn new(
        input_dir: &'a Path,
        cache_dir: &'a Path,
        extensions: &'a str,
        threads: usize,
        timeout: u32,
    ) -> Self {
        Self {
            input_dir,
            cache_dir,
            extensions,
            threads,
            timeout,
        }
    }

    pub fn process_all(&self, scan_results: &[PboScanResult]) -> Result<()> {
        debug!("Processing {} PBOs for extraction", scan_results.len());
        
        // Process each PBO
        let results: Vec<_> = scan_results
            .par_iter()
            .with_max_len(self.threads)
            .map(|result| {
                let process_result = self.process_pbo(result);
                (result, process_result)
            })
            .collect();
            
        // Count successes and failures
        let success_count = results.iter().filter(|(_, r)| r.is_ok()).count();
        let failure_count = results.len() - success_count;
        
        debug!("PBO processing complete:");
        debug!("  Total PBOs processed: {}", results.len());
        debug!("  Successful: {}", success_count);
        debug!("  Failed: {}", failure_count);
        
        Ok(())
    }

    fn process_pbo(&self, scan_result: &PboScanResult) -> Result<()> {
        debug!("Processing PBO: {}", scan_result.path.display());
        
        // If no matching files, skip processing
        if scan_result.expected_files.is_empty() {
            debug!("No matching files found in PBO, skipping: {}", scan_result.path.display());
            return Ok(());
        }

        // Prepare output directory
        let (_, output_dir) = self.prepare_output_dirs(scan_result)?;

        // Extract files
        match self.extract_pbo_files(scan_result, &output_dir) {
            Ok(_) => {
                debug!("Successfully extracted PBO to {}", output_dir.display());
            },
            Err(e) => {
                warn!("Failed to extract PBO {}: {}", scan_result.path.display(), e);
            }
        }

        Ok(())
    }

    fn prepare_output_dirs(&self, scan_result: &PboScanResult) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
        // Create output directory for this PBO
        let rel_path = scan_result.path.strip_prefix(self.input_dir)?;
        let base_dir = self.cache_dir.join(rel_path).with_extension("");
        debug!("Creating base directory: {}", base_dir.display());
        std::fs::create_dir_all(&base_dir)?;

        // Get prefix from PBO
        let api = self.create_pbo_api();
        
        // List contents and get prefix
        debug!("Listing contents of PBO: {}", scan_result.path.display());
        let list_result = match api.list_contents(&scan_result.path) {
            Ok(result) => result,
            Err(e) => {
                warn!("Failed to list PBO contents {}: {}", scan_result.path.display(), e);
                return Err(anyhow::anyhow!("Failed to list PBO contents: {}", e));
            }
        };
        
        let prefix = list_result.get_prefix().unwrap_or_default();
        debug!("PBO prefix: {}", prefix);

        // Create output directory with prefix path
        let output_dir = base_dir.join(prefix);
        trace!("Creating output directory: {}", output_dir.display());
        std::fs::create_dir_all(&output_dir)?;

        Ok((base_dir, output_dir))
    }

    fn create_pbo_api(&self) -> PboApi {
        let config = PboConfig::default();
        PboApi::builder()
            .with_config(config)
            .with_timeout(self.timeout)
            .build()
    }

    fn create_extract_options(&self) -> ExtractOptions {
        let mut options = ExtractOptions::default();
        options.file_filter = Some(self.extensions.split(',').map(str::to_string).collect());
        options.no_pause = true;
        options.warnings_as_errors = false;
        options.verbose = true;
        options
    }

    fn extract_pbo_files(
        &self, 
        scan_result: &PboScanResult, 
        output_dir: &std::path::Path
    ) -> Result<ExtractResult> {
        let api = self.create_pbo_api();
        let options = self.create_extract_options();
        
        // Attempt 1: Standard extraction
        debug!("Trying standard extraction for PBO: {}", scan_result.path.display());
        match api.extract_with_options(&scan_result.path, output_dir, options.clone()) {
            Ok(result) => {
                debug!("Extraction successful with standard extraction");
                Ok(result)
            },
            Err(e) => {
                warn!("Standard extraction failed: {}", e);
                
                // Attempt 2: Permissive extraction
                debug!("Trying permissive extraction for PBO: {}", scan_result.path.display());
                let mut permissive_options = options.clone();
                permissive_options.file_filter = None; // Extract all files
                match api.extract_with_options(&scan_result.path, output_dir, permissive_options) {
                    Ok(result) => {
                        debug!("Extraction successful with permissive extraction");
                        Ok(result)
                    },
                    Err(e) => {
                        warn!("Permissive extraction failed: {}", e);
                        
                        // Attempt 3: Direct extraction
                        debug!("Trying direct extraction for PBO: {}", scan_result.path.display());
                        match api.extract_files(&scan_result.path, output_dir, None) {
                            Ok(result) => {
                                debug!("Extraction successful with direct extraction");
                                Ok(result)
                            },
                            Err(e) => {
                                warn!("Direct extraction failed: {}", e);
                                Err(anyhow::anyhow!("All extraction attempts failed: {}", e))
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;
    
    #[test]
    fn test_skip_empty_result() {
        let input_dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        
        let scan_result = PboScanResult {
            path: PathBuf::from("test.pbo"),
            expected_files: vec![],
        };
        
        let processor = PboProcessor::new(
            input_dir.path(),
            cache_dir.path(),
            "sqf,hpp",
            1,
            30,
        );
        
        let result = processor.process_pbo(&scan_result);
        assert!(result.is_ok());
    }
}
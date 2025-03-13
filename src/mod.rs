use std::path::Path;
use anyhow::Result;
use log::debug;
use pbo_tools::{
    core::api::{PboApi, PboApiOps},
    extract::ExtractOptions,
};

use crate::scanner::coordinator::ScanCoordinator;

pub mod types;
pub mod scanner;
pub mod utils;

/// Configuration for the PBO extraction process
#[derive(Debug, Clone)]
pub struct ExtractionConfig<'a> {
    /// Directory containing PBO files to scan
    pub input_dir: &'a Path,
    /// Directory for extracted PBO contents
    pub output_dir: &'a Path,
    /// File extensions to extract (comma-separated)
    pub extensions: &'a str,
    /// Number of parallel threads to use
    pub threads: usize,
    /// Timeout in seconds for PBO operations
    pub timeout: u32,
}

/// Extract files from multiple PBO archives in parallel
///
/// # Arguments
/// * `config` - Configuration specifying input/output directories and extraction options
///
/// # Returns
/// * `Result<()>` - Success or error during extraction
pub async fn extract_pbos(config: ExtractionConfig<'_>) -> Result<()> {
    debug!("Starting PBO extraction with configuration:");
    debug!("  Input directory: {}", config.input_dir.display());
    debug!("  Output directory: {}", config.output_dir.display());
    debug!("  Extensions filter: {}", config.extensions);
    debug!("  Threads: {}", config.threads);
    debug!("  Timeout: {} seconds", config.timeout);
    
    // Verify input directory exists and is readable
    if !config.input_dir.exists() {
        return Err(anyhow::anyhow!("Input directory does not exist: {}", config.input_dir.display()));
    }
    
    // Verify input directory is readable by trying to list its contents
    if let Err(e) = std::fs::read_dir(config.input_dir) {
        return Err(anyhow::anyhow!("Input directory is not readable: {} - {}", config.input_dir.display(), e));
    }
    
    // Create output directory if it doesn't exist
    if !config.output_dir.exists() {
        debug!("Creating output directory: {}", config.output_dir.display());
        if let Err(e) = std::fs::create_dir_all(config.output_dir) {
            return Err(anyhow::anyhow!("Failed to create output directory: {} - {}", config.output_dir.display(), e));
        }
    }
    
    // Verify output directory is writable
    let test_file = config.output_dir.join(".test_write");
    if let Err(e) = std::fs::write(&test_file, "test") {
        return Err(anyhow::anyhow!("Output directory is not writable: {} - {}", config.output_dir.display(), e));
    }
    let _ = std::fs::remove_file(test_file);

    // Create and run the coordinator
    let coordinator = ScanCoordinator::new(
        config.input_dir,
        config.output_dir,
        config.extensions,
        config.threads,
        config.timeout,
    )?;

    coordinator.run().await
}

/// Extract a single PBO archive with default options
///
/// # Arguments
/// * `pbo_path` - Path to the PBO file
/// * `output_dir` - Directory where contents will be extracted
///
/// # Returns
/// * `Result<()>` - Success or error during extraction
pub fn extract_pbo(pbo_path: &Path, output_dir: &Path) -> Result<()> {
    let api = PboApi::builder()
        .with_timeout(30)
        .build();
    api.extract_files(pbo_path, output_dir, None)?;
    Ok(())
}

/// Extract a single PBO archive with custom options
///
/// # Arguments
/// * `pbo_path` - Path to the PBO file
/// * `output_dir` - Directory where contents will be extracted
/// * `options` - Custom extraction options
///
/// # Returns
/// * `Result<()>` - Success or error during extraction
pub fn extract_pbo_with_options(
    pbo_path: &Path,
    output_dir: &Path,
    options: ExtractOptions,
) -> Result<()> {
    let api = PboApi::builder()
        .with_timeout(30)
        .build();
    api.extract_with_options(pbo_path, output_dir, options)?;
    Ok(())
}

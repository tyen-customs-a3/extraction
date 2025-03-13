#[allow(dead_code)]
use std::path::Path;
use anyhow::Result;
use log::{debug, trace};
use pbo_tools::core::api::{PboApi, PboApiOps};
use pbo_tools::extract::ExtractOptions;

use super::types::PboScanResult;

/// Scan a PBO file for contents matching the specified extensions
pub fn scan_pbo_contents(
    path: &Path,
    extensions: &str,
    timeout: u32,
) -> Result<PboScanResult> {
    debug!("Scanning PBO contents: {}", path.display());
    debug!("Looking for extensions: {}", extensions);

    let api = PboApi::builder()
        .with_timeout(timeout)
        .build();

    let options = ExtractOptions {
        no_pause: true,
        warnings_as_errors: false,
        brief_listing: true,
        ..Default::default()
    };

    let result = api.list_with_options(path, options)?;
    let mut matching_files = Vec::new();

    debug!("Files in PBO:");
    for file in result.get_file_list() {
        trace!("  {}", file);
        let path = Path::new(&file);
        // Check if file matches extension filter
        if path.extension()
            .map(|ext| extensions.contains(&ext.to_string_lossy().to_string()))
            .unwrap_or(false)
        {
            trace!("    -> Matches extension filter");
            matching_files.push(file.to_string());
        }
    }

    debug!("Found {} matching files", matching_files.len());

    Ok(PboScanResult {
        path: path.to_owned(),
        expected_files: matching_files,
    })
}
use std::path::{Path, PathBuf};
use anyhow::Result;
use log::{info, warn};
use pbo_tools::core::api::{PboApi, PboApiOps};
use pbo_tools::extract::ExtractOptions;
use walkdir::WalkDir;

pub use crate::extraction::database::types::ScanDatabase;

pub mod types;
pub mod database;
pub mod scanner;
pub mod utils;

pub use database::*;
pub use scanner::*;

/// Configuration for the PBO extraction process
#[derive(Debug, Clone)]
pub struct ExtractionConfig<'a> {
    /// Directory containing PBO files to scan
    pub input_dir: &'a Path,
    /// Directory for caching extraction results
    pub cache_dir: &'a Path,
    /// File extensions to extract (comma-separated)
    pub extensions: &'a str,
    /// Number of parallel threads to use
    pub threads: usize,
    /// Timeout in seconds for PBO operations
    pub timeout: u32,
}

/// Main entry point for PBO extraction functionality
pub async fn extract_pbos(config: ExtractionConfig<'_>) -> Result<()> {
    info!("Starting PBO extraction with configuration:");
    info!("  Input directory: {}", config.input_dir.display());
    info!("  Cache directory: {}", config.cache_dir.display());
    info!("  Extensions filter: {}", config.extensions);
    info!("  Threads: {}", config.threads);
    info!("  Timeout: {} seconds", config.timeout);
    
    // Verify input directory exists
    if !config.input_dir.exists() {
        return Err(anyhow::anyhow!("Input directory does not exist: {}", config.input_dir.display()));
    }
    
    // Create cache directory if it doesn't exist
    if !config.cache_dir.exists() {
        info!("Creating cache directory: {}", config.cache_dir.display());
        std::fs::create_dir_all(config.cache_dir)?;
    }
    
    let coordinator = scanner::ScanCoordinator::new(
        config.input_dir,
        config.cache_dir,
        config.extensions,
        config.threads,
        config.timeout,
    )?;
    
    let result = coordinator.run().await;
    
    // Verify files were actually extracted
    if result.is_ok() {
        info!("Verifying extracted files in cache directory: {}", config.cache_dir.display());
        let extracted_file_count = WalkDir::new(config.cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();
        
        info!("Found {} files in cache directory", extracted_file_count);
        
        if extracted_file_count == 0 {
            warn!("No files were extracted to the cache directory");
        }
        
        // Verify that the database file exists
        let db_path = config.cache_dir.join("scan_db.json");
        if db_path.exists() {
            info!("Database file exists at: {}", db_path.display());
        } else {
            warn!("Database file not found at: {}", db_path.display());
        }
    }
    
    result
}

pub fn extract_pbo(pbo_path: &PathBuf, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let api = PboApi::builder()
        .with_timeout(30)
        .build();
    api.extract_files(pbo_path, output_dir, None)?;
    Ok(())
}

pub fn extract_pbo_with_options(
    pbo_path: &PathBuf,
    output_dir: &PathBuf,
    options: ExtractOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let api = PboApi::builder()
        .with_timeout(30)
        .build();
    api.extract_with_options(pbo_path, output_dir, options)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_extract_medical_pbo() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_pbo = PathBuf::from("src/tests/data/ace_medical.pbo");
        let output_dir = temp_dir.path().to_path_buf();

        // Extract the PBO
        let result = extract_pbo(&test_pbo, &output_dir);
        assert!(result.is_ok(), "Failed to extract PBO: {:?}", result.err());

        // Check that the expected files exist
        let medical_dir = output_dir.join("z/ace/addons/medical");
        let expected_files = vec![
            "ACE_Settings.hpp",
            "CfgEventHandlers.hpp",
            "CfgVehicles.hpp",
            "config.cpp",
            "script_component.hpp",
            "stringtable.xml",
            "XEH_PREP.hpp",
            "XEH_preInit.sqf",
            "XEH_postInit.sqf",
            "initSettings.inc.sqf",
        ];

        for file in expected_files {
            let file_path = medical_dir.join(file);
            assert!(file_path.exists(), "Expected file {} does not exist", file);
        }

        // Check that subdirectories exist
        let expected_dirs = vec!["dev", "functions", "ui"];
        for dir in expected_dirs {
            let dir_path = medical_dir.join(dir);
            assert!(dir_path.is_dir(), "Expected directory {} does not exist", dir);
        }

        // Check some specific files in subdirectories
        let function_files = vec![
            "functions/fnc_addDamageToUnit.sqf",
            "functions/fnc_adjustPainLevel.sqf",
            "functions/fnc_deserializeState.sqf",
            "functions/fnc_serializeState.sqf",
            "functions/fnc_setUnconscious.sqf",
        ];

        for file in function_files {
            let file_path = medical_dir.join(file);
            assert!(file_path.exists(), "Expected function file {} does not exist", file);
        }

        // Check UI files
        let ui_files = vec![
            "ui/tourniquet_arm_left.paa",
            "ui/tourniquet_arm_right.paa",
            "ui/tourniquet_leg_left.paa",
            "ui/tourniquet_leg_right.paa",
        ];

        for file in ui_files {
            let file_path = medical_dir.join(file);
            assert!(file_path.exists(), "Expected UI file {} does not exist", file);
        }
    }

    #[test]
    fn test_extract_with_filter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_pbo = PathBuf::from("src/tests/data/ace_medical.pbo");
        let output_dir = temp_dir.path().to_path_buf();

        // Extract only HPP files
        let options = ExtractOptions {
            file_filter: Some("hpp".to_string()),
            ..Default::default()
        };
        let result = extract_pbo_with_options(&test_pbo, &output_dir, options);
        assert!(result.is_ok(), "Failed to extract PBO with filter: {:?}", result.err());

        let medical_dir = output_dir.join("z/ace/addons/medical");
        let expected_hpp_files = vec![
            "ACE_Settings.hpp",
            "CfgEventHandlers.hpp",
            "CfgVehicles.hpp",
            "script_component.hpp",
            "XEH_PREP.hpp",
        ];

        // Check that only HPP files exist
        for file in expected_hpp_files {
            let file_path = medical_dir.join(file);
            assert!(file_path.exists(), "Expected HPP file {} does not exist", file);
        }

        // Check that SQF files don't exist
        let sqf_file = medical_dir.join("XEH_preInit.sqf");
        assert!(!sqf_file.exists(), "SQF file should not exist when filtering for HPP");
    }

    #[test]
    fn test_extract_unchanged_pbo() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_pbo = PathBuf::from("src/tests/data/ace_medical.pbo");
        let output_dir = temp_dir.path().to_path_buf();

        // First extraction
        let result = extract_pbo(&test_pbo, &output_dir);
        assert!(result.is_ok(), "Failed first extraction: {:?}", result.err());

        // Get file content after first extraction
        let medical_dir = output_dir.join("z/ace/addons/medical");
        let config_path = medical_dir.join("config.cpp");
        let first_content = fs::read(&config_path).expect("Failed to read first file");

        // Remove output directory and recreate it
        fs::remove_dir_all(&output_dir).expect("Failed to remove output directory");
        fs::create_dir_all(&output_dir).expect("Failed to create output directory");

        // Second extraction
        let result = extract_pbo(&test_pbo, &output_dir);
        assert!(result.is_ok(), "Failed second extraction: {:?}", result.err());

        // Check that files have the same content
        let second_content = fs::read(&config_path).expect("Failed to read second file");
        assert_eq!(
            first_content, second_content,
            "File contents differ between extractions"
        );

        // Check that the file exists and is readable
        assert!(config_path.exists(), "File does not exist after second extraction");
        assert!(config_path.is_file(), "Path is not a file after second extraction");
    }

    #[test]
    fn test_extract_multiple_pbos() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_pbo = PathBuf::from("src/tests/data/ace_medical.pbo");
        let output_dir = temp_dir.path().to_path_buf();

        // Extract the same PBO twice to different subdirectories
        let first_output = output_dir.join("first");
        let second_output = output_dir.join("second");

        fs::create_dir_all(&first_output).expect("Failed to create first output dir");
        fs::create_dir_all(&second_output).expect("Failed to create second output dir");

        let result = extract_pbo(&test_pbo, &first_output);
        assert!(result.is_ok(), "Failed first extraction: {:?}", result.err());

        let result = extract_pbo(&test_pbo, &second_output);
        assert!(result.is_ok(), "Failed second extraction: {:?}", result.err());

        // Check that both extractions have the same files
        let first_medical = first_output.join("z/ace/addons/medical");
        let second_medical = second_output.join("z/ace/addons/medical");

        let check_files = vec![
            "config.cpp",
            "script_component.hpp",
            "stringtable.xml",
            "functions/fnc_addDamageToUnit.sqf",
            "ui/tourniquet_arm_left.paa",
        ];

        for file in check_files {
            let first_path = first_medical.join(file);
            let second_path = second_medical.join(file);

            assert!(first_path.exists(), "File {} missing from first extraction", file);
            assert!(second_path.exists(), "File {} missing from second extraction", file);

            let first_content = fs::read(&first_path).expect("Failed to read first file");
            let second_content = fs::read(&second_path).expect("Failed to read second file");
            assert_eq!(
                first_content, second_content,
                "File {} contents differ between extractions", file
            );
        }
    }
}

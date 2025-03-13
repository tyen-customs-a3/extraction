use std::path::Path;
use anyhow::Result;
use log::info;
use env_logger;
use pbo_tools::core::api::{PboApi, PboApiOps};

const TEST_PBO_DIR: &str = "tests/fixtures";
const TEST_PBO: &str = "ace_medical.pbo";

// Expected files in the medical test fixture
const EXPECTED_FILES: &[&str] = &[
    // Root files
    "ACE_Settings.hpp",
    "CfgEventHandlers.hpp",
    "CfgVehicles.hpp",
    "config.cpp",
    "initSettings.inc.sqf",
    "script_component.hpp",
    "stringtable.xml",
    "XEH_postInit.sqf",
    "XEH_postInit.sqfc",
    "XEH_preInit.sqf",
    "XEH_preInit.sqfc",
    "XEH_PREP.hpp",
    "XEH_preStart.sqf",
    "XEH_preStart.sqfc",
    
    // Dev files
    "dev/debugDisplay.sqf",
    "dev/debugDisplay.sqfc",
    "dev/reportSettings.sqf",
    "dev/reportSettings.sqfc",
    "dev/test_hitpointConfigs.sqf",
    "dev/test_hitpointConfigs.sqfc",
    "dev/watchVariable.sqf",
    "dev/watchVariable.sqfc",
    
    // Function files
    "functions/fnc_addDamageToUnit.sqf",
    "functions/fnc_addDamageToUnit.sqfc",
    "functions/fnc_adjustPainLevel.sqf",
    "functions/fnc_adjustPainLevel.sqfc",
    "functions/fnc_deserializeState.sqf",
    "functions/fnc_deserializeState.sqfc",
    "functions/fnc_serializeState.sqf",
    "functions/fnc_serializeState.sqfc",
    "functions/fnc_setUnconscious.sqf",
    "functions/fnc_setUnconscious.sqfc",
    
    // UI files
    "ui/tourniquet_arm_left.paa",
    "ui/tourniquet_arm_right.paa",
    "ui/tourniquet_leg_left.paa",
    "ui/tourniquet_leg_right.paa",
];

fn setup_logging() {
    let _ = env_logger::try_init();
}

#[test]
fn test_list_pbo_contents() -> Result<()> {
    setup_logging();
    let input_dir = Path::new(TEST_PBO_DIR);
    let pbo_path = input_dir.join(TEST_PBO);
    info!("Testing PBO scanning from: {}", pbo_path.display());
    
    // Create PBO API and list contents
    let api = PboApi::builder()
        .with_timeout(30)
        .build();
    
    let options = pbo_tools::extract::ExtractOptions {
        no_pause: true,
        warnings_as_errors: false,
        brief_listing: false,
        ..Default::default()
    };
    
    let result = api.list_with_options(&pbo_path, options)?;
    let files = result.get_file_list();
    
    info!("Found {} files in PBO:", files.len());
    for file in files.iter() {
        info!("  {}", file);
    }
    
    // Verify all expected files are present
    for expected_file in EXPECTED_FILES {
        if !files.contains(&expected_file.to_string()) {
            info!("Missing expected file: {}", expected_file);
        }
        assert!(
            files.contains(&expected_file.to_string()),
            "Expected file {} was not found in PBO",
            expected_file
        );
    }
    
    // Check for unexpected files
    for file in files.iter() {
        if !EXPECTED_FILES.contains(&file.as_str()) {
            info!("Found unexpected file: {}", file);
        }
    }
    
    // Verify no unexpected files
    assert_eq!(
        files.len(),
        EXPECTED_FILES.len(),
        "Found {} files but expected {}",
        files.len(),
        EXPECTED_FILES.len()
    );
    
    Ok(())
}

#[test]
fn test_extract_with_options() -> Result<()> {
    setup_logging();
    let input_dir = Path::new(TEST_PBO_DIR);
    let pbo_path = input_dir.join(TEST_PBO);
    let output_dir = tempfile::tempdir()?;
    info!("Testing PBO extraction with options from: {}", pbo_path.display());
    
    // Create PBO API and extract with options
    let api = PboApi::builder()
        .with_timeout(30)
        .build();
    
    let mut options = pbo_tools::extract::ExtractOptions {
        no_pause: true,
        warnings_as_errors: false,
        brief_listing: false,
        ..Default::default()
    };
    options.file_filter = Some("hpp".to_string());
    
    api.extract_with_options(&pbo_path, output_dir.path(), options)?;
    
    // List all hpp files in the output directory
    let files: Vec<_> = walkdir::WalkDir::new(output_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "hpp"))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    
    info!("Found {} hpp files:", files.len());
    for file in &files {
        info!("  {}", file);
    }
    
    // We should only see the hpp files
    assert!(files.contains(&"ACE_Settings.hpp".to_string()));
    assert!(files.contains(&"CfgEventHandlers.hpp".to_string()));
    assert!(files.contains(&"CfgVehicles.hpp".to_string()));
    assert!(files.contains(&"script_component.hpp".to_string()));
    assert!(files.contains(&"XEH_PREP.hpp".to_string()));
    
    // Verify no other files were found
    assert_eq!(files.len(), 5);
    
    Ok(())
}

#[test]
fn test_extract_with_extension_filter() -> Result<()> {
    setup_logging();
    let input_dir = Path::new(TEST_PBO_DIR);
    let pbo_path = input_dir.join(TEST_PBO);
    let output_dir = tempfile::tempdir()?;
    info!("Testing PBO extraction with extension filter from: {}", pbo_path.display());
    
    // Create PBO API and extract with options
    let api = PboApi::builder()
        .with_timeout(30)
        .build();
    
    let mut options = pbo_tools::extract::ExtractOptions {
        no_pause: true,
        warnings_as_errors: false,
        brief_listing: false,
        ..Default::default()
    };
    options.file_filter = Some("hpp".to_string());
    
    api.extract_with_options(&pbo_path, output_dir.path(), options)?;
    
    // List all hpp files in the output directory
    let files: Vec<_> = walkdir::WalkDir::new(output_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "hpp"))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    
    info!("Found {} hpp files:", files.len());
    for file in &files {
        info!("  {}", file);
    }
    
    // We should only see the hpp files
    assert!(files.contains(&"ACE_Settings.hpp".to_string()));
    assert!(files.contains(&"CfgEventHandlers.hpp".to_string()));
    assert!(files.contains(&"CfgVehicles.hpp".to_string()));
    assert!(files.contains(&"script_component.hpp".to_string()));
    assert!(files.contains(&"XEH_PREP.hpp".to_string()));
    
    // Verify no other files were found
    assert_eq!(files.len(), 5);
    
    Ok(())
} 
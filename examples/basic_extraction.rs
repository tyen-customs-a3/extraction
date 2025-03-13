use std::path::PathBuf;
use anyhow::Result;
use extraction::{ExtractionConfig, extract_pbos};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Set up input and output directories
    let input_dir = PathBuf::from("./input");  // Directory containing PBO files
    let output_dir = PathBuf::from("./output"); // Directory where files will be extracted

    // Create the extraction configuration
    let config = ExtractionConfig {
        input_dir: &input_dir,
        output_dir: &output_dir,
        extensions: "sqf,hpp,cpp", // Extract files with these extensions
        threads: num_cpus::get(),  // Use all available CPU cores
        timeout: 30,               // 30 second timeout per PBO operation
    };

    // Run the extraction
    extract_pbos(config).await?;

    println!("Extraction complete!");
    Ok(())
}
use std::path::PathBuf;
use anyhow::Result;
use extraction::{
    ExtractionConfig, 
    extract_pbos, 
    extract_pbo,
    extract_pbo_with_options,
};
use pbo_tools::extract::ExtractOptions;
use log::{info, LevelFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up more detailed logging
    env_logger::Builder::new()
        .filter_level(LevelFilter::Debug)
        .init();

    // Example 1: Extract specific files from a single PBO
    info!("Example 1: Single PBO extraction");
    let single_pbo = PathBuf::from("./input/example.pbo");
    let single_output = PathBuf::from("./output/single");
    
    // Using default options
    extract_pbo(&single_pbo, &single_output)?;

    // Example 2: Extract with custom options
    info!("Example 2: Custom extraction options");
    let mut options = ExtractOptions::default();
    options.file_filter = Some("sqf,hpp,cpp".to_string());
    options.verbose = true;
    
    extract_pbo_with_options(
        &single_pbo,
        &PathBuf::from("./output/filtered"),
        options,
    )?;

    // Example 3: Batch processing with configuration
    info!("Example 3: Batch processing");
    let config = ExtractionConfig {
        input_dir: &PathBuf::from("./input"),
        output_dir: &PathBuf::from("./output/batch"),
        extensions: "sqf,hpp,cpp",
        threads: num_cpus::get(),
        timeout: 30,
    };

    extract_pbos(config).await?;

    println!("All examples completed successfully!");
    Ok(())
}
pub mod database;
pub mod scanner;
pub mod utils;
pub mod types;

// Include the module with the main implementation
#[path = "mod.rs"]
pub mod extraction;

// Re-export selected public items to avoid ambiguities
pub use extraction::{extract_pbo, extract_pbo_with_options, extract_pbos, ExtractionConfig};

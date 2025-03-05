use std::path::Path;
use anyhow::Result;
use super::types::{ScanDatabase, PboInfo, SkipReason, ScanStats};
use log;

impl ScanDatabase {
    pub fn load_or_create(path: &Path) -> Result<Self> {
        if path.exists() {
            log::debug!("Loading existing database from: {}", path.display());
            let file = std::fs::File::open(path)?;
            let db: Self = serde_json::from_reader(file)?;
            log::debug!("Loaded database with {} PBOs", db.pbos.len());
            Ok(db)
        } else {
            log::debug!("Database file not found, creating new database: {}", path.display());
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        log::debug!("Saving database with {} PBOs to: {}", self.pbos.len(), path.display());
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                log::debug!("Creating parent directory: {}", parent.display());
                std::fs::create_dir_all(parent)?;
            }
        }
        
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        log::debug!("Database saved successfully");
        Ok(())
    }

    pub fn get_pbo_info(&self, path: &Path) -> Option<&PboInfo> {
        let path_str = path.to_string_lossy().to_string();
        log::debug!("Looking up PBO in database: {}", path_str);
        
        let result = self.pbos.get(&path_str);
        if result.is_some() {
            log::debug!("  Found PBO in database");
        } else {
            log::debug!("  PBO not found in database");
        }
        
        result
    }

    pub fn update_pbo(&mut self, path: &Path, hash: &str, failed: bool) {
        self.pbos.insert(
            path.to_string_lossy().to_string(),
            PboInfo {
                hash: hash.to_string(),
                failed,
                skip_reason: None,
                expected_files: None,
                extracted_files: None,
            },
        );
    }

    pub fn update_pbo_with_reason(&mut self, path: &Path, hash: &str, failed: bool, reason: SkipReason) {
        self.pbos.insert(
            path.to_string_lossy().to_string(),
            PboInfo {
                hash: hash.to_string(),
                failed,
                skip_reason: Some(reason),
                expected_files: None,
                extracted_files: None,
            },
        );
    }

    pub fn update_pbo_with_files(
        &mut self, 
        path: &Path, 
        hash: &str, 
        expected_files: Vec<String>,
        extracted_files: Vec<String>,
    ) -> bool {
        // Consider extraction successful if any files were extracted
        // This is more lenient than requiring all expected files to be extracted
        let any_files_extracted = !extracted_files.is_empty();
        
        // For backward compatibility, still check if all expected files were extracted
        let all_files_extracted = expected_files.iter().all(|f| extracted_files.contains(f));
        
        // Convert path to string for storage
        let path_str = path.to_string_lossy().to_string();
        log::debug!("Updating database for PBO: {}", path_str);
        log::debug!("  Hash: {}", hash);
        log::debug!("  Expected files: {}", expected_files.len());
        log::debug!("  Extracted files: {}", extracted_files.len());
        log::debug!("  Any files extracted: {}", any_files_extracted);
        log::debug!("  All files extracted: {}", all_files_extracted);
        
        // Mark as successful if any files were extracted
        let is_failed = !any_files_extracted;
        let skip_reason = if any_files_extracted {
            if all_files_extracted {
                None // All files extracted, no reason to skip
            } else {
                // Some files were extracted, but not all
                // Still consider it a success, but note that some files were missing
                log::info!("Some expected files were not extracted from {}, but marking as successful", path_str);
                None
            }
        } else {
            // No files were extracted
            Some(SkipReason::MissingExpectedFiles)
        };
        
        self.pbos.insert(
            path_str,
            PboInfo {
                hash: hash.to_string(),
                failed: is_failed,
                skip_reason,
                expected_files: Some(expected_files),
                extracted_files: Some(extracted_files),
            },
        );
        
        // Return whether any files were extracted, not whether all files were extracted
        any_files_extracted
    }

    pub fn get_stats(&self) -> ScanStats {
        let mut stats = ScanStats {
            total: self.pbos.len(),
            ..Default::default()
        };

        for info in self.pbos.values() {
            if info.failed {
                match info.skip_reason {
                    Some(SkipReason::Empty) => stats.empty += 1,
                    Some(SkipReason::NoMatchingFiles) => stats.no_matching_files += 1,
                    Some(SkipReason::InvalidFormat) => stats.invalid_format += 1,
                    Some(SkipReason::Failed) => stats.failed += 1,
                    Some(SkipReason::MissingExpectedFiles) => stats.missing_expected_files += 1,
                    None => stats.failed += 1,
                }
            } else {
                stats.processed += 1;
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_load_or_create_new() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        
        let db = ScanDatabase::load_or_create(&path).unwrap();
        assert_eq!(db.pbos.len(), 0);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        
        let mut db = ScanDatabase::default();
        db.update_pbo(&PathBuf::from("/test/path.pbo").as_path(), "hash123", false);
        db.save(&path).unwrap();
        
        let loaded_db = ScanDatabase::load_or_create(&path).unwrap();
        assert_eq!(loaded_db.pbos.len(), 1);
        assert!(loaded_db.pbos.contains_key("/test/path.pbo"));
    }

    #[test]
    fn test_update_pbo_with_reason() {
        let mut db = ScanDatabase::default();
        db.update_pbo_with_reason(
            &PathBuf::from("/test/path.pbo").as_path(),
            "hash123",
            true,
            SkipReason::Empty,
        );
        
        let info = db.get_pbo_info(&PathBuf::from("/test/path.pbo").as_path()).unwrap();
        assert_eq!(info.hash, "hash123");
        assert!(info.failed);
        assert!(matches!(info.skip_reason, Some(SkipReason::Empty)));
    }

    #[test]
    fn test_update_pbo_with_files_all_extracted() {
        let mut db = ScanDatabase::default();
        let expected_files = vec![
            "file1.sqf".to_string(),
            "file2.sqf".to_string(),
        ];
        let extracted_files = vec![
            "file1.sqf".to_string(),
            "file2.sqf".to_string(),
            "extra.sqf".to_string(),  // Extra file is fine
        ];
        
        let result = db.update_pbo_with_files(
            &PathBuf::from("/test/path.pbo").as_path(),
            "hash123",
            expected_files,
            extracted_files,
        );
        
        assert!(result);
        let info = db.get_pbo_info(&PathBuf::from("/test/path.pbo").as_path()).unwrap();
        assert_eq!(info.hash, "hash123");
        assert!(!info.failed);
        assert!(info.skip_reason.is_none());
        assert_eq!(info.expected_files.as_ref().unwrap().len(), 2);
        assert_eq!(info.extracted_files.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_update_pbo_with_files_missing_files() {
        let mut db = ScanDatabase::default();
        let expected_files = vec![
            "file1.sqf".to_string(),
            "file2.sqf".to_string(),
            "file3.sqf".to_string(),
        ];
        let extracted_files = vec![
            "file1.sqf".to_string(),
            "file2.sqf".to_string(),
        ];
        
        let result = db.update_pbo_with_files(
            &PathBuf::from("/test/path.pbo").as_path(),
            "hash123",
            expected_files,
            extracted_files,
        );
        
        // With the new behavior, this should be true because some files were extracted
        assert!(result, "Result should be true if any files were extracted");
        
        let info = db.get_pbo_info(&PathBuf::from("/test/path.pbo").as_path()).unwrap();
        assert_eq!(info.hash, "hash123");
        
        // With the new behavior, this should be false because some files were extracted
        assert!(!info.failed, "PBO should not be marked as failed if any files were extracted");
        
        // With the new behavior, skip_reason should be None because some files were extracted
        assert!(info.skip_reason.is_none(), "Skip reason should be None if any files were extracted");
        
        // Verify the expected and extracted files are still recorded correctly
        assert_eq!(info.expected_files.as_ref().unwrap().len(), 3);
        assert_eq!(info.extracted_files.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_update_pbo_with_no_files_extracted() {
        let mut db = ScanDatabase::default();
        let expected_files = vec![
            "file1.sqf".to_string(),
            "file2.sqf".to_string(),
            "file3.sqf".to_string(),
        ];
        let extracted_files = vec![]; // No files extracted
        
        let result = db.update_pbo_with_files(
            &PathBuf::from("/test/path.pbo").as_path(),
            "hash123",
            expected_files,
            extracted_files,
        );
        
        // Result should be false because no files were extracted
        assert!(!result, "Result should be false if no files were extracted");
        
        let info = db.get_pbo_info(&PathBuf::from("/test/path.pbo").as_path()).unwrap();
        assert_eq!(info.hash, "hash123");
        
        // PBO should be marked as failed because no files were extracted
        assert!(info.failed, "PBO should be marked as failed if no files were extracted");
        
        // Skip reason should be MissingExpectedFiles because no files were extracted
        assert!(matches!(info.skip_reason, Some(SkipReason::MissingExpectedFiles)), 
                "Skip reason should be MissingExpectedFiles if no files were extracted");
        
        // Verify the expected and extracted files are still recorded correctly
        assert_eq!(info.expected_files.as_ref().unwrap().len(), 3);
        assert_eq!(info.extracted_files.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn test_get_stats() {
        let mut db = ScanDatabase::default();
        
        // Add various PBO statuses
        db.update_pbo(&PathBuf::from("/test/success.pbo").as_path(), "hash1", false);
        db.update_pbo_with_reason(&PathBuf::from("/test/empty.pbo").as_path(), "hash2", true, SkipReason::Empty);
        db.update_pbo_with_reason(&PathBuf::from("/test/no_files.pbo").as_path(), "hash3", true, SkipReason::NoMatchingFiles);
        db.update_pbo_with_reason(&PathBuf::from("/test/invalid.pbo").as_path(), "hash4", true, SkipReason::InvalidFormat);
        db.update_pbo_with_reason(&PathBuf::from("/test/failed.pbo").as_path(), "hash5", true, SkipReason::Failed);
        
        // Add a PBO with missing expected files but some files extracted (should be considered successful)
        let expected_files = vec!["file1.sqf".to_string(), "file2.sqf".to_string()];
        let extracted_files = vec!["file1.sqf".to_string()];
        db.update_pbo_with_files(
            &PathBuf::from("/test/missing.pbo").as_path(),
            "hash6",
            expected_files,
            extracted_files,
        );
        
        // Add a PBO with no files extracted (should be considered failed)
        let expected_files = vec!["file1.sqf".to_string(), "file2.sqf".to_string()];
        let extracted_files: Vec<String> = vec![];
        db.update_pbo_with_files(
            &PathBuf::from("/test/no_extracted.pbo").as_path(),
            "hash7",
            expected_files,
            extracted_files,
        );
        
        let stats = db.get_stats();
        assert_eq!(stats.total, 7);
        assert_eq!(stats.processed, 2); // success.pbo and missing.pbo (which has some files extracted)
        assert_eq!(stats.empty, 1);
        assert_eq!(stats.no_matching_files, 1);
        assert_eq!(stats.invalid_format, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.missing_expected_files, 1); // no_extracted.pbo
    }

    #[test]
    fn test_skip_unchanged_pbo() {
        // Create a database with an existing PBO entry
        let mut db = ScanDatabase::default();
        let pbo_path = PathBuf::from("/test/unchanged.pbo");
        let hash = "original_hash123";
        
        // Add the PBO to the database as successfully processed
        db.update_pbo(&pbo_path, hash, false);
        
        // Check if the PBO needs processing with the same hash
        let info = db.get_pbo_info(&pbo_path).unwrap();
        assert_eq!(info.hash, hash);
        assert!(!info.failed);
        
        // Verify that a PBO with the same hash doesn't need processing
        let needs_processing = match db.get_pbo_info(&pbo_path) {
            Some(info) if !info.failed && info.hash == hash => false,
            _ => true
        };
        
        assert!(!needs_processing, "PBO with unchanged hash should be skipped");
    }
    
    #[test]
    fn test_process_changed_pbo() {
        // Create a database with an existing PBO entry
        let mut db = ScanDatabase::default();
        let pbo_path = PathBuf::from("/test/changed.pbo");
        let original_hash = "original_hash123";
        
        // Add the PBO to the database as successfully processed
        db.update_pbo(&pbo_path, original_hash, false);
        
        // Check with a different hash (simulating a changed file)
        let new_hash = "new_hash456";
        let needs_processing = match db.get_pbo_info(&pbo_path) {
            Some(info) if !info.failed && info.hash == new_hash => false,
            _ => true
        };
        
        assert!(needs_processing, "PBO with changed hash should be processed");
    }
    
    #[test]
    fn test_process_previously_failed_pbo() {
        // Create a database with an existing PBO entry that failed
        let mut db = ScanDatabase::default();
        let pbo_path = PathBuf::from("/test/failed.pbo");
        let hash = "hash123";
        
        // Add the PBO to the database as failed
        db.update_pbo(&pbo_path, hash, true);
        
        // Check if the PBO needs processing with the same hash
        let needs_processing = match db.get_pbo_info(&pbo_path) {
            Some(info) if !info.failed && info.hash == hash => false,
            _ => true
        };
        
        assert!(needs_processing, "Previously failed PBO should be processed again");
    }
    
    #[test]
    fn test_skip_extraction_for_no_matching_files() {
        // Create a database with a PBO that has no matching files
        let mut db = ScanDatabase::default();
        let pbo_path = PathBuf::from("/test/no_matching.pbo");
        let hash = "hash123";
        
        // Mark the PBO as having no matching files
        db.update_pbo_with_reason(&pbo_path, hash, true, SkipReason::NoMatchingFiles);
        
        // Verify the PBO is marked correctly
        let info = db.get_pbo_info(&pbo_path).unwrap();
        assert!(info.failed);
        assert!(matches!(info.skip_reason, Some(SkipReason::NoMatchingFiles)));
        
        // This PBO should still be processed in the scan phase (to check if it changed)
        // but would be skipped in the extraction phase
        let needs_processing = match db.get_pbo_info(&pbo_path) {
            Some(info) if !info.failed && info.hash == hash => false,
            _ => true
        };
        
        assert!(needs_processing, "PBO with no matching files should be rescanned but extraction would be skipped");
    }
    
    #[test]
    fn test_target_folder_check_with_expected_files() {
        // Create a database with a PBO that has expected files
        let mut db = ScanDatabase::default();
        let pbo_path = PathBuf::from("/test/with_expected_files.pbo");
        let hash = "hash123";
        
        let expected_files = vec![
            "file1.sqf".to_string(),
            "file2.sqf".to_string(),
        ];
        let extracted_files = vec![
            "file1.sqf".to_string(),
            "file2.sqf".to_string(),
        ];
        
        // Update the database with the files information
        db.update_pbo_with_files(&pbo_path, hash, expected_files, extracted_files);
        
        // Verify the PBO is marked as successfully processed
        let info = db.get_pbo_info(&pbo_path).unwrap();
        assert!(!info.failed);
        assert!(info.skip_reason.is_none());
        assert_eq!(info.expected_files.as_ref().unwrap().len(), 2);
        assert_eq!(info.extracted_files.as_ref().unwrap().len(), 2);
        
        // This PBO should be skipped if the hash hasn't changed
        let needs_processing = match db.get_pbo_info(&pbo_path) {
            Some(info) if !info.failed && info.hash == hash => false,
            _ => true
        };
        
        assert!(!needs_processing, "PBO with all expected files extracted should be skipped if hash unchanged");
    }
    
    #[test]
    fn test_complete_extraction_workflow() {
        // Create a temporary database
        let mut db = ScanDatabase::default();
        
        // Setup test PBO paths
        let unchanged_pbo = PathBuf::from("/test/unchanged.pbo");
        let changed_pbo = PathBuf::from("/test/changed.pbo");
        let failed_pbo = PathBuf::from("/test/failed.pbo");
        let new_pbo = PathBuf::from("/test/new.pbo");
        
        // Initial state: add some PBOs to the database
        
        // 1. A successfully processed PBO with all files extracted
        let expected_files1 = vec!["file1.sqf".to_string(), "file2.sqf".to_string()];
        let extracted_files1 = vec!["file1.sqf".to_string(), "file2.sqf".to_string()];
        db.update_pbo_with_files(&unchanged_pbo, "hash1", expected_files1, extracted_files1);
        
        // 2. A PBO that previously had files but hash will change
        let expected_files2 = vec!["file1.sqf".to_string(), "file2.sqf".to_string()];
        let extracted_files2 = vec!["file1.sqf".to_string(), "file2.sqf".to_string()];
        db.update_pbo_with_files(&changed_pbo, "old_hash2", expected_files2, extracted_files2);
        
        // 3. A PBO that previously failed extraction
        db.update_pbo_with_reason(&failed_pbo, "hash3", true, SkipReason::Failed);
        
        // Now simulate the scan process
        
        // 1. Unchanged PBO - should be skipped
        let unchanged_hash = "hash1"; // Same hash as before
        let needs_processing1 = match db.get_pbo_info(&unchanged_pbo) {
            Some(info) if !info.failed && info.hash == unchanged_hash => false,
            _ => true
        };
        assert!(!needs_processing1, "Unchanged PBO should be skipped");
        
        // 2. Changed PBO - should be processed
        let new_hash = "new_hash2"; // Different hash
        let needs_processing2 = match db.get_pbo_info(&changed_pbo) {
            Some(info) if !info.failed && info.hash == new_hash => false,
            _ => true
        };
        assert!(needs_processing2, "Changed PBO should be processed");
        
        // 3. Previously failed PBO - should be processed
        let failed_hash = "hash3"; // Same hash as before
        let needs_processing3 = match db.get_pbo_info(&failed_pbo) {
            Some(info) if !info.failed && info.hash == failed_hash => false,
            _ => true
        };
        assert!(needs_processing3, "Previously failed PBO should be processed");
        
        // 4. New PBO - should be processed
        let new_pbo_hash = "hash4";
        let needs_processing4 = match db.get_pbo_info(&new_pbo) {
            Some(info) if !info.failed && info.hash == new_pbo_hash => false,
            _ => true
        };
        assert!(needs_processing4, "New PBO should be processed");
        
        // Now simulate updating the database after processing
        
        // 1. Update the changed PBO with new hash and files
        let expected_files_changed = vec!["file1.sqf".to_string(), "file2.sqf".to_string(), "file3.sqf".to_string()];
        let extracted_files_changed = vec!["file1.sqf".to_string(), "file2.sqf".to_string(), "file3.sqf".to_string()];
        db.update_pbo_with_files(&changed_pbo, new_hash, expected_files_changed, extracted_files_changed);
        
        // 2. Update the previously failed PBO as successful
        let expected_files_failed = vec!["file1.sqf".to_string()];
        let extracted_files_failed = vec!["file1.sqf".to_string()];
        db.update_pbo_with_files(&failed_pbo, failed_hash, expected_files_failed, extracted_files_failed);
        
        // 3. Add the new PBO
        let expected_files_new = vec!["file1.sqf".to_string(), "file2.sqf".to_string()];
        let extracted_files_new = vec!["file1.sqf".to_string(), "file2.sqf".to_string()];
        db.update_pbo_with_files(&new_pbo, new_pbo_hash, expected_files_new, extracted_files_new);
        
        // Verify final state
        
        // 1. Unchanged PBO should still be marked as successful
        let unchanged_info = db.get_pbo_info(&unchanged_pbo).unwrap();
        assert!(!unchanged_info.failed);
        assert_eq!(unchanged_info.hash, "hash1");
        
        // 2. Changed PBO should be updated with new hash and marked as successful
        let changed_info = db.get_pbo_info(&changed_pbo).unwrap();
        assert!(!changed_info.failed);
        assert_eq!(changed_info.hash, new_hash);
        assert_eq!(changed_info.expected_files.as_ref().unwrap().len(), 3);
        
        // 3. Previously failed PBO should now be marked as successful
        let failed_info = db.get_pbo_info(&failed_pbo).unwrap();
        assert!(!failed_info.failed);
        assert!(failed_info.skip_reason.is_none());
        
        // 4. New PBO should be in the database and marked as successful
        let new_info = db.get_pbo_info(&new_pbo).unwrap();
        assert!(!new_info.failed);
        assert_eq!(new_info.hash, new_pbo_hash);
        
        // Check stats
        let stats = db.get_stats();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.processed, 4); // All PBOs are now successfully processed
        assert_eq!(stats.failed, 0);
    }
} 
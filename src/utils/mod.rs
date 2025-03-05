use std::path::Path;
use sha2::{Sha256, Digest};
use std::fs::{File, metadata};
use std::io::Read;
use anyhow::Result;
use std::time::SystemTime;

/// Calculate a fast hash of a file based on metadata and partial content
/// 
/// This function creates a hash based on:
/// - File size
/// - Last modification time
/// - First 4KB of file content (or less if file is smaller)
/// 
/// This is much faster than hashing the entire file while still being
/// reasonably accurate for detecting changes.
pub fn calculate_file_hash(path: &Path) -> Result<String> {
    let meta = metadata(path)?;
    let file_size = meta.len();
    
    // Get modification time as seconds since UNIX epoch
    let modified = meta.modified()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let mut hasher = Sha256::new();
    
    // Add file metadata to hash
    hasher.update(file_size.to_string().as_bytes());
    hasher.update(modified.to_string().as_bytes());
    
    // Read first 4KB of file content
    let mut file = File::open(path)?;
    let mut buffer = [0; 4096];
    let bytes_read = file.read(&mut buffer)?;
    
    if bytes_read > 0 {
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}

/// Check if a file extension matches any in a comma-separated list
pub fn matches_extension(path: &Path, extensions: &str) -> bool {
    if extensions.is_empty() {
        return true;
    }

    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        extensions.split(',')
            .map(str::trim)
            .map(str::to_lowercase)
            .any(|e| e == ext_str)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_calculate_file_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create test file with known content
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();
        
        // Get the hash using our function
        let hash = calculate_file_hash(&file_path).unwrap();
        
        // Verify the hash is not empty and has the expected format (64 hex chars)
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        
        // Verify that the same file produces the same hash
        let hash2 = calculate_file_hash(&file_path).unwrap();
        assert_eq!(hash, hash2);
        
        // Verify that different content produces different hash
        let different_path = temp_dir.path().join("different.txt");
        let mut different_file = File::create(&different_path).unwrap();
        different_file.write_all(b"different content").unwrap();
        
        let different_hash = calculate_file_hash(&different_path).unwrap();
        assert_ne!(hash, different_hash);
    }

    #[test]
    fn test_matches_extension_empty_list() {
        let path = Path::new("test.txt");
        assert!(matches_extension(path, ""));
    }

    #[test]
    fn test_matches_extension_single() {
        let path = Path::new("test.txt");
        assert!(matches_extension(path, "txt"));
        assert!(!matches_extension(path, "cpp"));
    }

    #[test]
    fn test_matches_extension_multiple() {
        let path = Path::new("test.cpp");
        assert!(matches_extension(path, "txt,cpp,h"));
        assert!(!matches_extension(path, "txt,h,rs"));
    }

    #[test]
    fn test_matches_extension_case_insensitive() {
        let path = Path::new("test.TXT");
        assert!(matches_extension(path, "txt"));
        
        let path = Path::new("test.txt");
        assert!(matches_extension(path, "TXT"));
    }

    #[test]
    fn test_matches_extension_no_extension() {
        let path = Path::new("test");
        assert!(!matches_extension(path, "txt,cpp"));
    }

    #[test]
    fn test_matches_extension_whitespace() {
        let path = Path::new("test.txt");
        assert!(matches_extension(path, " txt , cpp "));
    }
} 
//! # Cache Module
//!
//! Provides SHA1-based caching for compiled script binaries.
//!
//! The cache key is computed from:
//! - Script file content
//! - Dependencies
//! - Script path (for relative resolution)
//!
//! Cache structure:
//! ```ignore
//! ~/.cache/pipeliner-script/
//!   <sha1-hash>/
//!     script.rs       # Original script
//!     script          # Compiled binary
//!     manifest        # Dependencies used
//! ```

use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Cache entry metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    /// SHA1 hash of the script content + dependencies
    pub hash: String,
    /// Path to the original script
    pub script_path: String,
    /// Dependencies used for compilation
    pub dependencies: Vec<String>,
    /// When the entry was created
    pub created_at: String,
    /// When the entry was last accessed
    pub last_accessed: String,
    /// Access count
    pub access_count: u64,
}

impl CacheEntry {
    /// Creates a new cache entry.
    #[must_use]
    pub fn new(hash: String, script_path: String, dependencies: Vec<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            hash,
            script_path,
            dependencies,
            created_at: now.clone(),
            last_accessed: now,
            access_count: 1,
        }
    }

    /// Updates the last accessed time and increments counter.
    pub fn touch(&mut self) {
        self.last_accessed = chrono::Utc::now().to_rfc3339();
        self.access_count += 1;
    }
}

/// Script cache for storing compiled binaries.
#[derive(Debug)]
pub struct ScriptCache {
    /// Root cache directory
    cache_dir: PathBuf,
    /// Maximum age for cache entries (None = never expire)
    max_age: Option<Duration>,
    /// Maximum number of entries (None = unlimited)
    max_entries: Option<usize>,
}

// Safety: ScriptCache is thread-safe because all operations are file-based
// and don't mutate the internal struct fields after construction.
unsafe impl Send for ScriptCache {}
unsafe impl Sync for ScriptCache {}

impl ScriptCache {
    /// Creates a new script cache with the given root directory.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the directory cannot be created.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self, CacheError> {
        let cache_dir = cache_dir.into();
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)
                .map_err(|e| CacheError::IoError(format!("Failed to create cache dir: {}", e)))?;
        }
        Ok(Self {
            cache_dir,
            max_age: None,
            max_entries: None,
        })
    }

    /// Creates a cache in the default location (~/.pipeliner/cache).
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the directory cannot be created.
    pub fn with_default_location() -> Result<Self, CacheError> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("pipeliner")
            .join("cache");
        Self::new(cache_dir)
    }

    /// Sets the maximum age for cache entries.
    #[must_use]
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = Some(max_age);
        self
    }

    /// Sets the maximum number of entries in the cache.
    #[must_use]
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = Some(max_entries);
        self
    }

    /// Computes the cache hash for a script.
    ///
    /// The hash is computed from:
    /// - Script content
    /// - Dependencies
    /// - Script path (for relative deps resolution)
    pub fn compute_hash(script_content: &str, dependencies: &[String], script_path: &Path) -> String {
        let mut hasher = Sha1::new();

        // Add script content
        hasher.update(script_content.as_bytes());

        // Add dependencies (sorted for consistency)
        let mut sorted_deps = dependencies.to_vec();
        sorted_deps.sort();
        for dep in sorted_deps {
            hasher.update(dep.as_bytes());
        }

        // Add script path for relative resolution
        hasher.update(script_path.to_string_lossy().as_bytes());

        // Finalize and return hex string
        hex::encode(hasher.finalize())
    }

    /// Returns the cache directory for a given hash.
    #[must_use]
    fn cache_path(&self, hash: &str) -> PathBuf {
        self.cache_dir.join(hash)
    }

    /// Returns the path to the cached binary for a hash.
    #[must_use]
    fn binary_path(&self, hash: &str) -> PathBuf {
        self.cache_path(hash).join("script")
    }

    /// Returns the path to the cached script for a hash.
    #[must_use]
    fn script_path(&self, hash: &str) -> PathBuf {
        self.cache_path(hash).join("script.rs")
    }

    /// Returns the path to the manifest file for a hash.
    #[must_use]
    fn manifest_path(&self, hash: &str) -> PathBuf {
        self.cache_path(hash).join("manifest.json")
    }

    /// Returns the path to the metadata file for a hash.
    #[must_use]
    fn metadata_path(&self, hash: &str) -> PathBuf {
        self.cache_path(hash).join("metadata.json")
    }

    /// Checks if a cache entry exists and is valid.
    #[must_use]
    pub fn contains(&self, hash: &str) -> bool {
        let binary = self.binary_path(hash);
        binary.exists() && fs::metadata(&binary).map(|m| m.is_file()).unwrap_or(false)
    }

    /// Gets the path to a cached binary if it exists.
    ///
    /// Returns `None` if the cache entry doesn't exist or is invalid.
    #[must_use]
    pub fn get(&self, hash: &str) -> Option<PathBuf> {
        let binary = self.binary_path(hash);
        if binary.exists() && fs::metadata(&binary).map(|m| m.is_file()).unwrap_or(false) {
            // Update access metadata
            if let Ok(mut entry) = self.load_metadata(hash) {
                entry.touch();
                let _ = self.save_metadata(hash, &entry);
            }
            Some(binary)
        } else {
            None
        }
    }

    /// Stores a compiled binary in the cache.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the binary cannot be stored.
    pub fn store(
        &self,
        hash: String,
        script_content: &str,
        binary_path: impl AsRef<Path>,
        dependencies: &[String],
    ) -> Result<(), CacheError> {
        let cache_path = self.cache_path(&hash);

        // Create cache directory
        if !cache_path.exists() {
            fs::create_dir_all(&cache_path)
                .map_err(|e| CacheError::IoError(format!("Failed to create cache dir: {}", e)))?;
        }

        // Copy binary
        let dest_binary = self.binary_path(&hash);
        fs::copy(binary_path.as_ref(), &dest_binary)
            .map_err(|e| CacheError::IoError(format!("Failed to copy binary: {}", e)))?;

        // Save original script
        fs::write(self.script_path(&hash), script_content)
            .map_err(|e| CacheError::IoError(format!("Failed to save script: {}", e)))?;

        // Save dependencies manifest
        let manifest_json = serde_json::to_string_pretty(dependencies)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;
        fs::write(self.manifest_path(&hash), manifest_json)
            .map_err(|e| CacheError::IoError(format!("Failed to save manifest: {}", e)))?;

        // Save metadata
        let entry = CacheEntry::new(hash.clone(), "script.rs".to_string(), dependencies.to_vec());
        self.save_metadata(&hash, &entry)?;

        // Enforce max entries
        if let Some(max) = self.max_entries {
            self.evict_if_needed(max)?;
        }

        Ok(())
    }

    /// Removes a cache entry.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the entry cannot be removed.
    pub fn remove(&self, hash: &str) -> Result<(), CacheError> {
        let cache_path = self.cache_path(hash);
        if cache_path.exists() {
            fs::remove_dir_all(&cache_path)
                .map_err(|e| CacheError::IoError(format!("Failed to remove cache entry: {}", e)))?;
        }
        Ok(())
    }

    /// Lists all cache entries.
    pub fn list(&self) -> Result<Vec<CacheEntry>, CacheError> {
        let mut entries = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(entries);
        }

        for entry in fs::read_dir(&self.cache_dir)
            .map_err(|e| CacheError::IoError(format!("Failed to read cache dir: {}", e)))?
        {
            let entry = entry.map_err(|e| CacheError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(hash) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(metadata) = self.load_metadata(hash) {
                        entries.push(metadata);
                    }
                }
            }
        }

        // Sort by last accessed (most recent first)
        entries.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

        Ok(entries)
    }

    /// Clears all entries from the cache.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the cache cannot be cleared.
    pub fn clear(&self) -> Result<(), CacheError> {
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)
                .map_err(|e| CacheError::IoError(format!("Failed to read cache dir: {}", e)))?
            {
                let entry = entry.map_err(|e| CacheError::IoError(e.to_string()))?;
                fs::remove_dir_all(entry.path())
                    .map_err(|e| CacheError::IoError(format!("Failed to remove entry: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Returns the number of entries in the cache.
    #[must_use]
    pub fn len(&self) -> usize {
        self.list().map(|v| v.len()).unwrap_or(0)
    }

    /// Returns true if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total size of the cache in bytes.
    pub fn size(&self) -> Result<u64, CacheError> {
        let mut total = 0u64;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        for entry in walkdir::WalkDir::new(&self.cache_dir) {
            let entry = entry.map_err(|e| CacheError::IoError(e.to_string()))?;
            if entry.file_type().is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }

        Ok(total)
    }

    // =====================================================================
    // Private helpers
    // =====================================================================

    fn load_metadata(&self, hash: &str) -> Result<CacheEntry, CacheError> {
        let path = self.metadata_path(hash);
        let content = fs::read_to_string(&path)
            .map_err(|e| CacheError::IoError(format!("Failed to read metadata: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| CacheError::SerializationError(format!("Failed to parse metadata: {}", e)))
    }

    fn save_metadata(&self, hash: &str, entry: &CacheEntry) -> Result<(), CacheError> {
        let path = self.metadata_path(hash);
        let content = serde_json::to_string_pretty(entry)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;
        fs::write(&path, content)
            .map_err(|e| CacheError::IoError(format!("Failed to write metadata: {}", e)))
    }

    fn evict_if_needed(&self, max_entries: usize) -> Result<(), CacheError> {
        let entries = self.list()?;

        if entries.len() > max_entries {
            // Remove least recently accessed entries
            let to_remove = entries.len() - max_entries;
            for entry in entries.iter().rev().take(to_remove) {
                self.remove(&entry.hash)?;
            }
        }

        // Also check max age and remove expired entries
        if let Some(max_age) = self.max_age {
            let now = chrono::Utc::now();
            for entry in entries {
                if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&entry.created_at) {
                    let created = created.with_timezone(&chrono::Utc);
                    let age = now.signed_duration_since(created);
                    if age.to_std().map(|d| d > max_age).unwrap_or(false) {
                        self.remove(&entry.hash)?;
                    }
                }
            }
        }

        Ok(())
    }
}

/// Cache-related errors.
#[derive(Debug, Clone)]
pub enum CacheError {
    /// I/O error
    IoError(String),
    /// Serialization error
    SerializationError(String),
    /// Cache entry not found
    NotFound(String),
    /// Invalid cache entry
    InvalidEntry(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::IoError(msg) => write!(f, "Cache I/O error: {}", msg),
            CacheError::SerializationError(msg) => write!(f, "Cache serialization error: {}", msg),
            CacheError::NotFound(hash) => write!(f, "Cache entry not found: {}", hash),
            CacheError::InvalidEntry(msg) => write!(f, "Invalid cache entry: {}", msg),
        }
    }
}

impl std::error::Error for CacheError {}

impl From<std::io::Error> for CacheError {
    fn from(err: std::io::Error) -> Self {
        CacheError::IoError(err.to_string())
    }
}

// =======================================================================
// Tests
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_hash_deterministic() {
        let content = "fn main() {}";
        let deps = vec!["serde = \"1.0\"".to_string()];
        let path = Path::new("script.rs");

        let hash1 = ScriptCache::compute_hash(content, &deps, path);
        let hash2 = ScriptCache::compute_hash(content, &deps, path);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_different_content() {
        let deps = vec!["serde = \"1.0\"".to_string()];
        let path = Path::new("script.rs");

        let hash1 = ScriptCache::compute_hash("fn main() {}", &deps, path);
        let hash2 = ScriptCache::compute_hash("fn main() { println!(); }", &deps, path);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_different_deps() {
        let content = "fn main() {}";
        let path = Path::new("script.rs");

        let hash1 = ScriptCache::compute_hash(content, &["serde = \"1.0\"".to_string()], path);
        let hash2 = ScriptCache::compute_hash(content, &["tokio = \"1.0\"".to_string()], path);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_cache_new_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");

        let cache = ScriptCache::new(&cache_dir).unwrap();
        assert!(cache_dir.exists());
    }

    #[test]
    fn test_cache_store_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let mut cache = ScriptCache::new(&cache_dir).unwrap();

        // Create a fake binary
        let binary = temp_dir.path().join("fake_binary");
        std::fs::write(&binary, "fake binary content").unwrap();

        let content = "fn main() {}";
        let deps = vec!["serde = \"1.0\"".to_string()];
        let hash = ScriptCache::compute_hash(content, &deps, Path::new("script.rs"));

        // Store
        cache.store(hash.clone(), content, &binary, &deps).unwrap();

        // Get
        let cached = cache.get(&hash);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), cache.binary_path(&hash));
    }

    #[test]
    fn test_cache_contains() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let mut cache = ScriptCache::new(&cache_dir).unwrap();

        let binary = temp_dir.path().join("fake_binary");
        std::fs::write(&binary, "fake").unwrap();

        let content = "fn main() {}";
        let deps = vec![];
        let hash = ScriptCache::compute_hash(content, &deps, Path::new("script.rs"));

        assert!(!cache.contains(&hash));

        cache.store(hash.clone(), content, &binary, &deps).unwrap();

        assert!(cache.contains(&hash));
    }

    #[test]
    fn test_cache_remove() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let mut cache = ScriptCache::new(&cache_dir).unwrap();

        let binary = temp_dir.path().join("fake_binary");
        std::fs::write(&binary, "fake").unwrap();

        let hash = ScriptCache::compute_hash("fn main() {}", &[], Path::new("script.rs"));

        cache.store(hash.clone(), "fn main() {}", &binary, &[]).unwrap();
        assert!(cache.contains(&hash));

        cache.remove(&hash).unwrap();
        assert!(!cache.contains(&hash));
    }

    #[test]
    fn test_cache_list() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let mut cache = ScriptCache::new(&cache_dir).unwrap();

        let binary = temp_dir.path().join("fake_binary");
        std::fs::write(&binary, "fake").unwrap();

        for i in 0..3 {
            let content = format!("fn main() {{}} {}", i);
            let hash = ScriptCache::compute_hash(&content, &[], Path::new("script.rs"));
            cache.store(hash, &content, &binary, &[]).unwrap();
        }

        let entries = cache.list().unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let mut cache = ScriptCache::new(&cache_dir).unwrap();

        let binary = temp_dir.path().join("fake_binary");
        std::fs::write(&binary, "fake").unwrap();

        cache.store("hash1".to_string(), "fn main() {}", &binary, &[]).unwrap();
        cache.store("hash2".to_string(), "fn main() {}", &binary, &[]).unwrap();

        assert_eq!(cache.len(), 2);

        cache.clear().unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_with_max_entries() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let binary = temp_dir.path().join("fake_binary");
        std::fs::write(&binary, "fake").unwrap();

        // Create cache with max 2 entries
        let mut cache = ScriptCache::new(&cache_dir)
            .unwrap()
            .with_max_entries(2);

        // Add 3 entries
        for i in 0..3 {
            let content = format!("fn main() {{}} {}", i);
            let hash = ScriptCache::compute_hash(&content, &[], Path::new("script.rs"));
            cache.store(hash, &content, &binary, &[]).unwrap();
        }

        // Should only have 2 entries
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_entry_touch() {
        let mut entry = CacheEntry::new(
            "abc123".to_string(),
            "script.rs".to_string(),
            vec!["serde = \"1.0\"".to_string()],
        );

        assert_eq!(entry.access_count, 1);

        entry.touch();
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn test_cache_error_display() {
        let err = CacheError::IoError("disk full".to_string());
        assert!(err.to_string().contains("disk full"));

        let err = CacheError::NotFound("abc123".to_string());
        assert!(err.to_string().contains("abc123"));
    }
}
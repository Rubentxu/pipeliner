//! Library artifact discovery types.

use std::path::{Path, PathBuf};

use crate::LibraryError;

/// Discovered library artifacts from a library source.
///
/// This struct holds the paths to all relevant files discovered
/// when loading a library from a source (git, local path, or local lib).
#[derive(Debug, Clone, Default)]
pub struct LibraryArtifacts {
    /// Source files discovered (e.g., Rust source in src/)
    pub source_files: Vec<PathBuf>,
    /// Step files discovered (e.g., custom step definitions in steps/)
    pub step_files: Vec<PathBuf>,
    /// Resource files discovered (e.g., static resources in resources/)
    pub resource_files: Vec<PathBuf>,
}

impl LibraryArtifacts {
    /// Creates a new empty LibraryArtifacts
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if no artifacts were discovered
    pub fn is_empty(&self) -> bool {
        self.source_files.is_empty()
            && self.step_files.is_empty()
            && self.resource_files.is_empty()
    }

    /// Returns the total number of files discovered
    pub fn total_files(&self) -> usize {
        self.source_files.len() + self.step_files.len() + self.resource_files.len()
    }

    /// Returns true if any step files were discovered
    pub fn has_step_files(&self) -> bool {
        !self.step_files.is_empty()
    }

    /// Discovers library artifacts by walking the directory structure.
    ///
    /// Scans for files in:
    /// - `src/` subdirectory → source files
    /// - `steps/` subdirectory → step definition files
    /// - `resources/` subdirectory → static resource files
    ///
    /// # Arguments
    ///
    /// * `base` - The base directory to search in
    ///
    /// # Returns
    ///
    /// Returns `Ok(LibraryArtifacts)` with discovered file paths, or `Err(LibraryError)` on failure
    pub fn discover_from(base: &Path) -> Result<Self, LibraryError> {
        use walkdir::WalkDir;

        // Check if base directory exists
        if !base.exists() {
            return Err(LibraryError::SourceNotFound(base.display().to_string()));
        }

        let mut artifacts = LibraryArtifacts::new();

        // Discover source files from src/
        let src_dir = base.join("src");
        if src_dir.exists() {
            for entry in WalkDir::new(&src_dir)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    artifacts.source_files.push(entry.path().to_path_buf());
                }
            }
        }

        // Discover step files from steps/
        let steps_dir = base.join("steps");
        if steps_dir.exists() {
            for entry in WalkDir::new(&steps_dir)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    artifacts.step_files.push(entry.path().to_path_buf());
                }
            }
        }

        // Discover resource files from resources/
        let resources_dir = base.join("resources");
        if resources_dir.exists() {
            for entry in WalkDir::new(&resources_dir)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    artifacts.resource_files.push(entry.path().to_path_buf());
                }
            }
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::TempDir;

    // ===================================================================
    // B1: LibraryArtifacts Struct Definition Tests (RED → GREEN)
    // ===================================================================

    #[test]
    fn test_library_artifacts_default() {
        // B1: LibraryArtifacts has Default
        let artifacts = LibraryArtifacts::default();
        assert!(artifacts.source_files.is_empty());
        assert!(artifacts.step_files.is_empty());
        assert!(artifacts.resource_files.is_empty());
    }

    #[test]
    fn test_library_artifacts_new() {
        // B1: LibraryArtifacts::new() creates empty artifacts
        let artifacts = LibraryArtifacts::new();
        assert!(artifacts.is_empty());
        assert_eq!(artifacts.total_files(), 0);
    }

    #[test]
    fn test_library_artifacts_with_files() {
        // B1: LibraryArtifacts can hold file paths
        let mut artifacts = LibraryArtifacts::new();
        artifacts.source_files.push(PathBuf::from("src/lib.rs"));
        artifacts.step_files.push(PathBuf::from("steps/my_step.yaml"));
        artifacts.resource_files.push(PathBuf::from("resources/config.json"));

        assert!(!artifacts.is_empty());
        assert_eq!(artifacts.total_files(), 3);
        assert_eq!(artifacts.source_files.len(), 1);
        assert_eq!(artifacts.step_files.len(), 1);
        assert_eq!(artifacts.resource_files.len(), 1);
    }

    // ===================================================================
    // B2: discover_from Tests (RED → GREEN → TRIANGULATE)
    // ===================================================================

    #[test]
    fn test_discover_from_valid_layout() {
        // SCN-LS-003: LocalSource resolves existing directory with src/, steps/, resources/
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create directory structure
        fs::create_dir_all(base.join("src")).expect("Should create src dir");
        fs::create_dir_all(base.join("steps")).expect("Should create steps dir");
        fs::create_dir_all(base.join("resources")).expect("Should create resources dir");

        // Create files
        fs::write(base.join("src/lib.rs"), "pub mod foo;").expect("Should create src/lib.rs");
        fs::write(base.join("src/main.rs"), "fn main() {}").expect("Should create src/main.rs");
        fs::write(base.join("steps/deploy.yaml"), "name: deploy").expect("Should create steps/deploy.yaml");
        fs::write(base.join("resources/app.conf"), "{}").expect("Should create resources/app.conf");

        let artifacts = LibraryArtifacts::discover_from(base).expect("Should discover artifacts");

        assert_eq!(artifacts.source_files.len(), 2);
        assert_eq!(artifacts.step_files.len(), 1);
        assert_eq!(artifacts.resource_files.len(), 1);
        assert!(!artifacts.is_empty());
        assert_eq!(artifacts.total_files(), 4);
    }

    #[test]
    fn test_discover_from_empty_directory() {
        // B3: SCN-LS-003 variant - empty dir returns empty artifacts
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        let artifacts = LibraryArtifacts::discover_from(base).expect("Should discover artifacts");

        assert!(artifacts.is_empty());
        assert_eq!(artifacts.total_files(), 0);
    }

    #[test]
    fn test_discover_from_missing_subdirectories() {
        // B3: SCN-LS-003 variant - missing subdirs are handled gracefully
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Only create src, no steps or resources
        fs::create_dir_all(base.join("src")).expect("Should create src dir");
        fs::write(base.join("src/lib.rs"), "pub mod foo;").expect("Should create src/lib.rs");

        let artifacts = LibraryArtifacts::discover_from(base).expect("Should discover artifacts");

        assert_eq!(artifacts.source_files.len(), 1);
        assert!(artifacts.step_files.is_empty());
        assert!(artifacts.resource_files.is_empty());
        assert!(!artifacts.is_empty());
    }

    #[test]
    fn test_discover_from_nonexistent_directory() {
        // B3: SCN-LS-004: Nonexistent path returns SourceNotFound error
        let result = LibraryArtifacts::discover_from(Path::new("/nonexistent/path/to/lib"));
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::error::LibraryError::SourceNotFound(_) ));
    }

    #[test]
    fn test_discover_from_nested_files() {
        // B3: SCN-LS-005: Recursive discovery within subdirectories
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create nested source structure
        fs::create_dir_all(base.join("src/foo/bar")).expect("Should create nested dirs");
        fs::write(base.join("src/lib.rs"), "pub mod foo;").expect("Should create file");
        fs::write(base.join("src/foo/mod.rs"), "pub mod bar;").expect("Should create file");
        fs::write(base.join("src/foo/bar/baz.rs"), "pub struct Baz;").expect("Should create file");

        let artifacts = LibraryArtifacts::discover_from(base).expect("Should discover artifacts");

        assert_eq!(artifacts.source_files.len(), 3);
    }

    // ===================================================================
    // B4: Helper Method Tests (RED → GREEN)
    // ===================================================================

    #[test]
    fn test_is_empty_true() {
        let artifacts = LibraryArtifacts::new();
        assert!(artifacts.is_empty());
    }

    #[test]
    fn test_is_empty_false_with_source_files() {
        let mut artifacts = LibraryArtifacts::new();
        artifacts.source_files.push(PathBuf::from("src/lib.rs"));
        assert!(!artifacts.is_empty());
    }

    #[test]
    fn test_total_files_counts_all() {
        let mut artifacts = LibraryArtifacts::new();
        artifacts.source_files.push(PathBuf::from("a.rs"));
        artifacts.source_files.push(PathBuf::from("b.rs"));
        artifacts.step_files.push(PathBuf::from("step.yaml"));
        artifacts.resource_files.push(PathBuf::from("res.json"));

        assert_eq!(artifacts.total_files(), 4);
    }

    // ===================================================================
    // B4: Helper Method Tests (TRIANGULATE)
    // ===================================================================

    #[test]
    fn test_has_step_files_true() {
        let mut artifacts = LibraryArtifacts::new();
        artifacts.step_files.push(PathBuf::from("steps/deploy.yaml"));
        assert!(artifacts.has_step_files());
    }

    #[test]
    fn test_has_step_files_false() {
        let artifacts = LibraryArtifacts::new();
        assert!(!artifacts.has_step_files());
    }

    #[test]
    fn test_has_step_files_false_with_only_source_files() {
        let mut artifacts = LibraryArtifacts::new();
        artifacts.source_files.push(PathBuf::from("src/lib.rs"));
        assert!(!artifacts.has_step_files());
    }
}

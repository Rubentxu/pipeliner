//! Temp file management for Jenkins compatibility
//!
//! This module provides Jenkins-compatible temp file management with support for:
//!
//! - `@tmp/` - Temporary files cleaned by Jenkins
//! - `@libs/` - Shared persistent files
//! - `@script@libs/` - Pipeline-specific persistent files

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Temporary file manager with Jenkins-compatible structure
///
/// # Example
///
/// ```rust
/// use rustline::TempFileManager;
/// use std::io::Write;
/// use tempfile::TempDir;
///
/// let temp_dir = TempDir::new().unwrap();
/// let manager = TempFileManager::new(temp_dir.path(), "my-job", "123").unwrap();
///
/// // Create temp file
/// let mut temp_file = manager.create_temp_file("script.sh").unwrap();
/// writeln!(temp_file, "#!/bin/sh\necho hello").unwrap();
/// drop(temp_file);
///
/// // Create shared file
/// let mut shared = manager.create_libs_file("config.json").unwrap();
/// let content = r#"{"key":"value"}"#;
/// writeln!(shared, "{}", content).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct TempFileManager {
    /// Workspace root directory
    workspace: PathBuf,

    /// Job name
    job_name: String,

    /// Build ID
    build_id: String,

    /// Temp directory path
    tmp_dir: PathBuf,

    /// Libs directory path
    libs_dir: PathBuf,

    /// Script libs directory path
    script_libs_dir: PathBuf,
}

impl TempFileManager {
    /// Creates a new temp file manager
    ///
    /// # Arguments
    ///
    /// * `workspace` - Workspace root directory
    /// * `job_name` - Name of the Jenkins job
    /// * `build_id` - Build ID (usually BUILD_NUMBER)
    ///
    /// # Returns
    ///
    /// A new `TempFileManager` with all directories created
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if directories cannot be created
    pub fn new(
        workspace: impl Into<PathBuf>,
        job_name: &str,
        build_id: &str,
    ) -> std::io::Result<Self> {
        let workspace = workspace.into();
        let tmp_dir = workspace.join("@tmp");
        let libs_dir = workspace.join("@libs");
        let script_libs_dir = workspace.join("@script@libs");

        // Create all directories
        fs::create_dir_all(&tmp_dir)?;
        fs::create_dir_all(&libs_dir)?;
        fs::create_dir_all(&script_libs_dir)?;

        Ok(Self {
            workspace,
            job_name: job_name.to_string(),
            build_id: build_id.to_string(),
            tmp_dir,
            libs_dir,
            script_libs_dir,
        })
    }

    /// Creates a new temp file in `@tmp/` directory
    ///
    /// # Arguments
    ///
    /// * `name` - Base name for the temp file
    ///
    /// # Returns
    ///
    /// A `File` handle for writing to the temp file
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file cannot be created
    pub fn create_temp_file(&self, name: &str) -> std::io::Result<File> {
        let unique_name = format!("{}-{}-{}", self.job_name, self.build_id, Uuid::new_v4());
        let file_path = self.tmp_dir.join(&unique_name);
        File::create(file_path)
    }

    /// Creates a temp file and returns its path
    ///
    /// # Arguments
    ///
    /// * `name` - Base name for the temp file
    /// * `content` - Content to write to the file
    ///
    /// # Returns
    ///
    /// The path to the created temp file
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file cannot be created or written
    pub fn create_temp_file_with_content(
        &self,
        name: &str,
        content: &str,
    ) -> std::io::Result<PathBuf> {
        let file_path = self.tmp_dir.join(format!(
            "{}-{}-{}",
            self.job_name,
            self.build_id,
            Uuid::new_v4()
        ));
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Creates a shared persistent file in `@libs/`
    ///
    /// These files are not cleaned by Jenkins and persist across builds.
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the shared file
    ///
    /// # Returns
    ///
    /// A `File` handle for writing to the shared file
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file cannot be created
    pub fn create_libs_file(&self, name: &str) -> std::io::Result<File> {
        let file_path = self.libs_dir.join(name);
        File::create(file_path)
    }

    /// Creates a shared file and returns its path
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the shared file
    /// * `content` - Content to write
    ///
    /// # Returns
    ///
    /// The path to the created shared file
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file cannot be created or written
    pub fn create_libs_file_with_content(
        &self,
        name: &str,
        content: &str,
    ) -> std::io::Result<PathBuf> {
        let file_path = self.libs_dir.join(name);
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Creates a pipeline-specific persistent file in `@script@libs/`
    ///
    /// These files persist for the duration of the pipeline run but are
    /// cleaned after the build completes.
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the pipeline-specific file
    ///
    /// # Returns
    ///
    /// A `File` handle for writing
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file cannot be created
    pub fn create_script_libs_file(&self, name: &str) -> std::io::Result<File> {
        let file_path = self.script_libs_dir.join(name);
        File::create(file_path)
    }

    /// Creates a script libs file and returns its path
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the file
    /// * `content` - Content to write
    ///
    /// # Returns
    ///
    /// The path to the created file
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file cannot be created or written
    pub fn create_script_libs_file_with_content(
        &self,
        name: &str,
        content: &str,
    ) -> std::io::Result<PathBuf> {
        let file_path = self.script_libs_dir.join(name);
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Reads content from a temp file
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the temp file (without path)
    ///
    /// # Returns
    ///
    /// The file contents as a string
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file cannot be read
    pub fn read_temp_file(&self, name: &str) -> std::io::Result<String> {
        let pattern = format!("{}-{}-{}", self.job_name, self.build_id, name);
        for entry in fs::read_dir(&self.tmp_dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().contains(&pattern) {
                return fs::read_to_string(entry.path());
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Temp file not found: {}", name),
        ))
    }

    /// Reads content from a libs file
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the libs file
    ///
    /// # Returns
    ///
    /// The file contents as a string
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the file cannot be read
    pub fn read_libs_file(&self, name: &str) -> std::io::Result<String> {
        let file_path = self.libs_dir.join(name);
        fs::read_to_string(file_path)
    }

    /// Cleans all temp files for the current build
    ///
    /// This is called automatically when the pipeline completes,
    /// but can also be called manually to clean up early.
    pub fn cleanup_temp_files(&self) -> std::io::Result<()> {
        let pattern = format!("{}-{}", self.job_name, self.build_id);

        if self.tmp_dir.exists() {
            for entry in fs::read_dir(&self.tmp_dir)? {
                let entry = entry?;
                let file_name = entry.file_name();
                if file_name.to_string_lossy().starts_with(&pattern) {
                    fs::remove_file(entry.path())?;
                }
            }
        }

        Ok(())
    }

    /// Cleans script libs files for the current build
    ///
    /// Unlike `@tmp/`, these files persist during the build and are
    /// cleaned up after completion.
    pub fn cleanup_script_libs(&self) -> std::io::Result<()> {
        if self.script_libs_dir.exists() {
            for entry in fs::read_dir(&self.script_libs_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    fs::remove_file(entry.path())?;
                }
            }
        }

        Ok(())
    }

    /// Full cleanup - cleans all temp and script libs files
    ///
    /// This should be called when the pipeline completes.
    pub fn full_cleanup(&self) -> std::io::Result<()> {
        self.cleanup_temp_files()?;
        self.cleanup_script_libs()?;
        Ok(())
    }

    /// Gets the path to the workspace directory
    #[must_use]
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// Gets the path to the temp directory
    #[must_use]
    pub fn tmp_dir(&self) -> &Path {
        &self.tmp_dir
    }

    /// Gets the path to the libs directory
    #[must_use]
    pub fn libs_dir(&self) -> &Path {
        &self.libs_dir
    }

    /// Gets the path to the script libs directory
    #[must_use]
    pub fn script_libs_dir(&self) -> &Path {
        &self.script_libs_dir
    }
}

impl Drop for TempFileManager {
    fn drop(&mut self) {
        // Auto-cleanup on drop
        let _ = self.full_cleanup();
    }
}

/// Jenkins-like path resolver for special directories
///
/// Resolves paths like `@tmp/file` or `@libs/config.json`
/// to their full workspace paths.
#[derive(Debug, Clone)]
pub struct JenkinsPathResolver {
    /// Workspace root
    workspace: PathBuf,

    /// Temp directory (for resolution)
    tmp: PathBuf,

    /// Libs directory (for resolution)
    libs: PathBuf,

    /// Script libs directory (for resolution)
    script_libs: PathBuf,
}

impl JenkinsPathResolver {
    /// Creates a new path resolver
    ///
    /// # Arguments
    ///
    /// * `workspace` - Workspace root directory
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        let workspace = workspace.into();
        let tmp = workspace.join("@tmp");
        let libs = workspace.join("@libs");
        let script_libs = workspace.join("@script@libs");

        Self {
            workspace,
            tmp,
            libs,
            script_libs,
        }
    }

    /// Resolves a Jenkins-style path to an absolute path
    ///
    /// # Arguments
    ///
    /// * `path` - The path to resolve (e.g., `@tmp/file.txt`)
    ///
    /// # Returns
    ///
    /// The resolved absolute path
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustline::JenkinsPathResolver;
    /// use tempfile::TempDir;
    ///
    /// let temp_dir = TempDir::new().unwrap();
    /// let resolver = JenkinsPathResolver::new(temp_dir.path());
    ///
    /// let path = resolver.resolve("@tmp/output.txt");
    /// assert!(path.to_string_lossy().contains("@tmp"));
    /// ```
    pub fn resolve(&self, path: &str) -> PathBuf {
        if path.starts_with("@tmp/") {
            self.tmp.join(&path[5..])
        } else if path.starts_with("@libs/") {
            self.libs.join(&path[6..])
        } else if path.starts_with("@script@libs/") {
            self.script_libs.join(&path[13..])
        } else if path.starts_with("@libs") {
            self.libs.join(&path[5..])
        } else if path.starts_with('@') {
            // Handle @tmp, @libs without trailing slash
            let rest = &path[1..];
            if rest.starts_with("tmp") {
                self.tmp.clone()
            } else if rest.starts_with("libs") {
                self.libs.clone()
            } else if rest.starts_with("script@libs") {
                self.script_libs.clone()
            } else {
                PathBuf::from(path)
            }
        } else {
            // Regular path - resolve relative to workspace
            self.workspace.join(path)
        }
    }

    /// Checks if a path is a special Jenkins path
    #[must_use]
    pub fn is_jenkins_path(path: &str) -> bool {
        path.starts_with("@tmp") || path.starts_with("@libs") || path.starts_with("@script@libs")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_temp_file_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TempFileManager::new(temp_dir.path(), "test-job", "42").unwrap();

        assert!(manager.tmp_dir().exists());
        assert!(manager.libs_dir().exists());
        assert!(manager.script_libs_dir().exists());
    }

    #[test]
    fn test_create_temp_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TempFileManager::new(temp_dir.path(), "test-job", "42").unwrap();

        let mut file = manager.create_temp_file("test.sh").unwrap();
        writeln!(file, "#!/bin/sh").unwrap();

        let pattern = format!("{}-{}", manager.job_name, manager.build_id);
        let mut found = false;
        for entry in fs::read_dir(manager.tmp_dir()).unwrap() {
            let entry = entry.unwrap();
            if entry.file_name().to_string_lossy().starts_with(&pattern) {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_create_libs_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TempFileManager::new(temp_dir.path(), "test-job", "42").unwrap();

        let mut file = manager.create_libs_file("shared.txt").unwrap();
        writeln!(file, "shared content").unwrap();

        let shared_path = manager.libs_dir().join("shared.txt");
        assert!(shared_path.exists());

        let content = fs::read_to_string(shared_path).unwrap();
        assert_eq!(content, "shared content\n");
    }

    #[test]
    fn test_cleanup_temp_files() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TempFileManager::new(temp_dir.path(), "cleanup-test", "100").unwrap();

        // Create temp files
        manager
            .create_temp_file_with_content("file1.txt", "content1")
            .unwrap();
        manager
            .create_temp_file_with_content("file2.txt", "content2")
            .unwrap();

        // Verify they exist
        assert!(manager.tmp_dir().read_dir().unwrap().next().is_some());

        // Cleanup
        manager.cleanup_temp_files().unwrap();

        // Should be empty
        assert!(manager.tmp_dir().read_dir().unwrap().next().is_none());
    }

    #[test]
    fn test_jenkins_path_resolver() {
        let temp_dir = TempDir::new().unwrap();
        let resolver = JenkinsPathResolver::new(temp_dir.path());

        assert_eq!(
            resolver.resolve("@tmp/output.txt"),
            temp_dir.path().join("@tmp/output.txt")
        );
        assert_eq!(
            resolver.resolve("@libs/config.json"),
            temp_dir.path().join("@libs/config.json")
        );
        assert_eq!(
            resolver.resolve("@script@libs/script.sh"),
            temp_dir.path().join("@script@libs/script.sh")
        );
    }

    #[test]
    fn test_jenkins_path_resolver_regular_path() {
        let temp_dir = TempDir::new().unwrap();
        let resolver = JenkinsPathResolver::new(temp_dir.path());

        assert_eq!(
            resolver.resolve("src/main.rs"),
            temp_dir.path().join("src/main.rs")
        );
    }

    #[test]
    fn test_is_jenkins_path() {
        assert!(JenkinsPathResolver::is_jenkins_path("@tmp/file"));
        assert!(JenkinsPathResolver::is_jenkins_path("@libs/file"));
        assert!(JenkinsPathResolver::is_jenkins_path("@script@libs/file"));
        assert!(!JenkinsPathResolver::is_jenkins_path("src/file.rs"));
        assert!(!JenkinsPathResolver::is_jenkins_path("/absolute/path"));
    }

    #[test]
    fn test_full_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TempFileManager::new(temp_dir.path(), "full-cleanup-test", "200").unwrap();

        // Create temp files
        manager
            .create_temp_file_with_content("temp.txt", "temp")
            .unwrap();
        manager
            .create_script_libs_file_with_content("script.txt", "script")
            .unwrap();

        // Verify they exist
        assert!(manager.tmp_dir().read_dir().unwrap().next().is_some());
        assert!(
            manager
                .script_libs_dir()
                .read_dir()
                .unwrap()
                .next()
                .is_some()
        );

        // Full cleanup
        manager.full_cleanup().unwrap();

        // Both should be empty
        assert!(manager.tmp_dir().read_dir().unwrap().next().is_none());
        assert!(
            manager
                .script_libs_dir()
                .read_dir()
                .unwrap()
                .next()
                .is_none()
        );
    }
}

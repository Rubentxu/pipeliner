//! Skill loading from Markdown files

use anyhow::{Context, Result};
use std::path::Path;

/// Load skill content from a Markdown file
///
/// Skills are Markdown files that provide additional context and instructions
/// for the agent. They follow the Agent Skills standard.
pub fn load_skill(skill_path: &Option<String>) -> Result<String> {
    match skill_path {
        Some(path) => load_skill_file(path),
        None => Ok(String::new()),
    }
}

/// Load a skill file from the given path
fn load_skill_file(path: &str) -> Result<String> {
    let path = Path::new(path);

    // Try relative to current dir first
    if path.exists() {
        return std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read skill file: {}", path.display()));
    }

    // Try relative to working directory
    let cwd = std::env::current_dir()
        .context("Failed to get current directory")?;
    let full_path = cwd.join(path);

    if full_path.exists() {
        return std::fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read skill file: {}", full_path.display()));
    }

    anyhow::bail!("Skill file not found: {}", path.display())
}

/// Validate that a skill path is valid
pub fn validate_skill(skill_path: &Option<String>) -> Result<()> {
    match skill_path {
        Some(path) => {
            let path = Path::new(path);
            if !path.exists() {
                anyhow::bail!("Skill file not found: {}", path.display());
            }
            if let Some(ext) = path.extension() {
                if ext != "md" {
                    tracing::warn!("Skill file should be .md, got: {}", ext.display());
                }
            }
            Ok(())
        }
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_load_skill_none() {
        let result = load_skill(&None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_load_skill_file() {
        let temp_dir = TempDir::new().unwrap();
        let skill_path = temp_dir.path().join("test-skill.md");
        
        let mut file = std::fs::File::create(&skill_path).unwrap();
        writeln!(file, "# Test Skill").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "This is a test skill.").unwrap();

        let result = load_skill(&Some(skill_path.to_string_lossy().to_string()));
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("# Test Skill"));
        assert!(content.contains("This is a test skill"));
    }

    #[test]
    fn test_load_skill_not_found() {
        let result = load_skill(&Some("/nonexistent/path/skill.md".to_string()));
        assert!(result.is_err());
    }
}

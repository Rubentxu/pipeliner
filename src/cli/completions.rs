//! `rustline completions` - Generate shell completions
//!
//! This module provides shell completion generation for rustline.
//! Supports bash, zsh, fish, and PowerShell.

use anyhow::{Context, Result};
use clap_complete::Shell;
use std::fs;
use std::path::{Path, PathBuf};

pub fn generate_completions(shell: Shell) -> Result<String> {
    use clap_complete::generate;

    let mut cmd = super::build_cli();
    let mut buf = Vec::new();
    generate(shell, &mut cmd, "rustline", &mut buf);

    String::from_utf8(buf).context("Failed to generate completions")
}

pub fn save_completions(completions: &str, output_path: &Path) -> Result<()> {
    fs::write(output_path, completions)
        .with_context(|| format!("Failed to write completions to: {}", output_path.display()))?;
    Ok(())
}

pub fn get_default_completions_path(shell: Shell) -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;

    match shell {
        Shell::Bash => {
            let path = format!("{}/.bash_completion.d/rustline", home);
            Ok(PathBuf::from(path))
        }
        Shell::Zsh => {
            let completion_dir = format!("{}/.zsh/completion", home);
            std::fs::create_dir_all(&completion_dir).ok();
            Ok(PathBuf::from(format!("{}/_rustline", completion_dir)))
        }
        Shell::Fish => {
            let completion_dir = format!("{}/.config/fish/completions", home);
            std::fs::create_dir_all(&completion_dir).ok();
            Ok(PathBuf::from(format!("{}/rustline.fish", completion_dir)))
        }
        Shell::PowerShell => {
            let profile = format!(
                "{}/Documents/PowerShell/Microsoft.PowerShell_profile.ps1",
                home
            );
            Ok(PathBuf::from(profile))
        }
        Shell::Elvish => {
            let path = format!("{}/.elvish/rc.elv", home);
            Ok(PathBuf::from(path))
        }
        _ => {
            anyhow::bail!("Unsupported shell: {:?}", shell);
        }
    }
}

pub fn list_available_shells() -> Vec<(Shell, &'static str, &'static str)> {
    vec![
        (Shell::Bash, "bash", "Bash (most Linux/macOS systems)"),
        (Shell::Zsh, "zsh", "Zsh (popular on macOS)"),
        (Shell::Fish, "fish", "Fish (user-friendly shell)"),
        (
            Shell::PowerShell,
            "powershell",
            "PowerShell (Windows/multi-platform)",
        ),
        (Shell::Elvish, "elvish", "Elvish (experimental shell)"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bash_completions() {
        let completions = generate_completions(Shell::Bash).unwrap();
        assert!(!completions.is_empty());
        assert!(completions.contains("rustline"));
    }

    #[test]
    fn test_generate_zsh_completions() {
        let completions = generate_completions(Shell::Zsh).unwrap();
        assert!(!completions.is_empty());
        assert!(completions.contains("rustline"));
    }

    #[test]
    fn test_generate_fish_completions() {
        let completions = generate_completions(Shell::Fish).unwrap();
        assert!(!completions.is_empty());
        assert!(completions.contains("rustline"));
    }

    #[test]
    fn test_list_available_shells() {
        let shells = list_available_shells();
        assert!(!shells.is_empty());
        assert!(shells.iter().any(|(s, _, _)| *s == Shell::Bash));
        assert!(shells.iter().any(|(s, _, _)| *s == Shell::Zsh));
    }
}

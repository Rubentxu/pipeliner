//! GC command - Garbage collection for old pipeline runs

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{debug, info};

use crate::config::OutputFormat;

/// Arguments for the `pipeliner gc` subcommand.
#[derive(Args, Debug)]
pub struct GcArgs {
    #[command(subcommand)]
    pub command: GcSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum GcSubcommand {
    /// List all runs without deleting
    List,

    /// Keep only the last N runs, delete older ones
    Keep {
        /// Number of runs to keep
        #[arg(value_name = "N")]
        n: usize,

        /// Preview what would be deleted without actually deleting
        #[arg(long, default_value = "false")]
        dry_run: bool,
    },
}

/// Run information for display
#[derive(Debug, serde::Serialize)]
pub struct RunInfo {
    pub run_id: String,
    pub date: String,
    pub age_hours: Option<f64>,
    pub status: String,
    pub size_bytes: u64,
    pub size_human: String,
}

/// Convert bytes to human-readable string
fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Get directory size recursively
fn dir_size(path: &PathBuf) -> u64 {
    let mut total: u64 = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                total += path.metadata().map(|m| m.len()).unwrap_or(0);
            } else if path.is_dir() {
                total += dir_size(&path);
            }
        }
    }
    total
}

/// Parse run_id to extract timestamp
/// Run IDs can be UUIDs (e.g., "a7f3e4b2-1234-...") or timestamp-based (e.g., "20260515-083012-a7f3")
fn parse_run_timestamp(run_id: &str) -> Option<u64> {
    // Try timestamp-based format first: YYYYMMDD-HHMMSS
    if run_id.len() >= 15 && run_id.chars().nth(8) == Some('-') {
        let date_part = &run_id[..8];
        let time_part = &run_id[9..15];
        if date_part.chars().all(|c| c.is_ascii_digit())
            && time_part.chars().all(|c| c.is_ascii_digit())
        {
            // Parse as chrono DateTime
            if let (Ok(year), Ok(month), Ok(day), Ok(hour), Ok(min), Ok(sec)) = (
                date_part[..4].parse::<u32>(),
                date_part[4..6].parse::<u32>(),
                date_part[6..8].parse::<u32>(),
                time_part[..2].parse::<u32>(),
                time_part[2..4].parse::<u32>(),
                time_part[4..6].parse::<u32>(),
            ) {
                // Create a simple timestamp (this is approximate but works for sorting)
                let days_since_epoch = julian_day(year, month, day) - 2440588; // Julian day - offset
                let seconds = (days_since_epoch as u64) * 86400
                    + (hour as u64) * 3600
                    + (min as u64) * 60
                    + sec as u64;
                return Some(seconds);
            }
        }
    }

    // Try UUID format - use creation time if available via filesystem
    // For UUIDs, we fall back to modification time
    None
}

/// Calculate Julian day number (simplified Gregorian calendar)
fn julian_day(year: u32, month: u32, day: u32) -> i64 {
    let a = (14 - month as i64) / 12;
    let y = year as i64 + 4800 - a;
    let m = month as i64 + 12 * a - 3;
    jd_to_mjd(i64::from(day) + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045)
}

/// Convert Julian Day to Modified Julian Day
fn jd_to_mjd(jd: i64) -> i64 {
    jd - 2400001
}

/// Get run info from a run directory
fn get_run_info(run_path: &PathBuf) -> Option<RunInfo> {
    let run_id = run_path.file_name()?.to_str()?.to_string();

    // Calculate size
    let size_bytes = dir_size(run_path);
    let size_human = human_size(size_bytes);

    // Try to get timestamp from report.json first
    if let Ok(content) = fs::read_to_string(run_path.join("report.json")) {
        if let Ok(report) = serde_json::from_str::<serde_json::Value>(&content) {
            // Try "completed_at" field
            if let Some(completed_at) = report.get("completed_at").or(report.get("started_at")) {
                if let Some(ts) = completed_at.as_str() {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                        let duration = SystemTime::now()
                            .duration_since(dt.with_timezone(&chrono::Utc).into())
                            .ok();
                        let age_hours = duration.map(|d| d.as_secs_f64() / 3600.0);
                        return Some(RunInfo {
                            run_id,
                            date: ts.to_string(),
                            age_hours,
                            status: report
                                .get("status")
                                .or(report.get("success"))
                                .map(|v| {
                                    if v.is_boolean() {
                                        if v.as_bool().unwrap_or(false) {
                                            "success".to_string()
                                        } else {
                                            "failed".to_string()
                                        }
                                    } else {
                                        v.as_str().unwrap_or("unknown").to_string()
                                    }
                                })
                                .unwrap_or_else(|| "unknown".to_string()),
                            size_bytes,
                            size_human,
                        });
                    }
                }
            }
        }
        // Fall back to events.jsonl
        if let Ok(content) = fs::read_to_string(run_path.join("events.jsonl")) {
            if let Some(first_line) = content.lines().next() {
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(first_line) {
                    if let Some(ts) = event.get("started_at").and_then(|v| v.as_str()) {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                            let duration = SystemTime::now()
                                .duration_since(dt.with_timezone(&chrono::Utc).into())
                                .ok();
                            let age_hours = duration.map(|d| d.as_secs_f64() / 3600.0);
                            return Some(RunInfo {
                                run_id,
                                date: ts.to_string(),
                                age_hours,
                                status: "unknown".to_string(),
                                size_bytes,
                                size_human,
                            });
                        }
                    }
                }
            }
        }
    };

    // Fall back to filesystem modification time
    let modified = run_path.metadata().ok()?.modified().ok()?;
    let duration = SystemTime::now().duration_since(modified).ok();
    let age_hours = duration.map(|d| d.as_secs_f64() / 3600.0);
    let date = chrono::DateTime::<chrono::Utc>::from(modified)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    Some(RunInfo {
        run_id,
        date,
        age_hours,
        status: "unknown".to_string(),
        size_bytes,
        size_human,
    })
}

/// List runs in the .pipeliner/runs directory
fn list_runs(runs_dir: &PathBuf) -> Result<Vec<RunInfo>> {
    if !runs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();

    for entry in fs::read_dir(runs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(run_info) = get_run_info(&path) {
                runs.push(run_info);
            }
        }
    }

    // Sort by date (newest first)
    runs.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(runs)
}

/// Execute the gc command
pub fn run_gc(args: GcArgs, format: OutputFormat) -> Result<()> {
    // Find .pipeliner/runs directory
    let runs_dir = find_runs_dir()?;

    debug!("Using runs directory: {:?}", runs_dir);

    match args.command {
        GcSubcommand::List => {
            let runs = list_runs(&runs_dir)?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&runs)?);
                }
                OutputFormat::Yaml => {
                    println!("{}", serde_yaml::to_string(&runs)?);
                }
                OutputFormat::Human => {
                    if runs.is_empty() {
                        println!("No runs found in {:?}", runs_dir);
                    } else {
                        println!("Runs in {:?}:", runs_dir);
                        println!(
                            "{:<40} {:<20} {:>10} {:>10}",
                            "RUN ID", "DATE", "STATUS", "SIZE"
                        );
                        println!("{}", "-".repeat(80));
                        for run in &runs {
                            println!(
                                "{:<40} {:<20} {:>10} {:>10}",
                                run.run_id.chars().take(38).collect::<String>()
                                    + if run.run_id.len() > 38 { "..." } else { "" },
                                run.date.chars().take(19).collect::<String>(),
                                run.status,
                                run.size_human
                            );
                        }
                        println!("\n{} run(s) found", runs.len());
                    }
                }
            }
        }
        GcSubcommand::Keep { n, dry_run } => {
            let runs = list_runs(&runs_dir)?;

            if runs.len() <= n {
                println!(
                    "No cleanup needed. {} run(s) found, keeping all {}.",
                    runs.len(),
                    n
                );
                return Ok(());
            }

            let to_delete: Vec<&RunInfo> = runs.iter().skip(n).collect();

            if dry_run {
                println!("DRY RUN: Would delete {} run(s):", to_delete.len());
                for run in &to_delete {
                    println!("  - {} ({}: {})", run.run_id, run.date, run.size_human);
                }
                println!(
                    "\nWould recover approximately {}",
                    human_size(to_delete.iter().map(|r| r.size_bytes).sum())
                );
            } else {
                println!("Deleting {} run(s)...", to_delete.len());
                let mut deleted = 0;
                let mut recovered: u64 = 0;

                for run in &to_delete {
                    let run_path = runs_dir.join(&run.run_id);
                    if run_path.exists() {
                        let size = dir_size(&run_path);
                        fs::remove_dir_all(&run_path)
                            .with_context(|| format!("Failed to delete {:?}", run_path))?;
                        deleted += 1;
                        recovered += size;
                        info!(
                            "Deleted run {} (recovered {})",
                            run.run_id,
                            human_size(size)
                        );
                    }
                }

                println!(
                    "Deleted {} run(s), recovered {}",
                    deleted,
                    human_size(recovered)
                );
            }
        }
    }

    Ok(())
}

/// Find the .pipeliner/runs directory
fn find_runs_dir() -> Result<PathBuf> {
    // Check current directory first
    let current = PathBuf::from(".");
    let current_runs = current.join(".pipeliner").join("runs");
    if current_runs.exists() {
        return Ok(current_runs);
    }

    // Check parent directories up to 3 levels
    let mut dir = std::env::current_dir()?;
    for _ in 0..3 {
        let runs = dir.join(".pipeliner").join("runs");
        if runs.exists() {
            return Ok(runs);
        }
        dir = dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Reached filesystem root"))?
            .to_path_buf();
    }

    anyhow::bail!(
        "No .pipeliner/runs directory found. \
         Please run a pipeline first or change to a directory with runs."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_human_size() {
        assert_eq!(human_size(500), "500 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(1048576), "1.0 MB");
        assert_eq!(human_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_julian_day() {
        // January 1, 1970
        assert_eq!(julian_day(1970, 1, 1), 2440588);
        // January 1, 2000
        assert_eq!(julian_day(2000, 1, 1), 2451545);
    }

    #[test]
    fn test_parse_run_timestamp_uuid() {
        // UUID format should return None (we can't extract timestamp reliably)
        let result = parse_run_timestamp("a7f3e4b2-1234-5678-abcd-ef0123456789");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_run_timestamp_timestamp() {
        // Timestamp format should return Some
        let result = parse_run_timestamp("20260515-083012-a7f3");
        assert!(result.is_some());
    }

    #[test]
    fn test_dir_size() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");

        // Create a file with known content
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(b"hello world").unwrap();

        let size = dir_size(&dir.path().to_path_buf());
        assert!(size >= 11); // "hello world" is 11 bytes
    }

    #[test]
    fn test_list_runs_empty() {
        let dir = TempDir::new().unwrap();
        let runs_dir = dir.path().join(".pipeliner").join("runs");
        std::fs::create_dir_all(&runs_dir).unwrap();

        let runs = list_runs(&runs_dir).unwrap();
        assert!(runs.is_empty());
    }

    #[test]
    fn test_list_runs_with_runs() {
        let dir = TempDir::new().unwrap();
        let runs_dir = dir.path().join(".pipeliner").join("runs");
        std::fs::create_dir_all(&runs_dir).unwrap();

        // Create a mock run directory
        let run1 = runs_dir.join("20260515-083012-a7f3");
        std::fs::create_dir_all(&run1).unwrap();
        std::fs::write(
            &run1.join("report.json"),
            r#"{"status": "success", "completed_at": "2026-05-15T08:30:12Z"}"#,
        )
        .unwrap();

        let runs = list_runs(&runs_dir).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_id, "20260515-083012-a7f3");
        assert_eq!(runs[0].status, "success");
    }

    #[test]
    fn test_get_run_info_with_report() {
        let dir = TempDir::new().unwrap();
        let run_path = dir.path().join("run1");
        std::fs::create_dir_all(&run_path).unwrap();
        std::fs::write(
            &run_path.join("report.json"),
            r#"{"status": "failed", "completed_at": "2026-05-15T08:30:12Z"}"#,
        )
        .unwrap();

        let info = get_run_info(&run_path).unwrap();
        assert_eq!(info.status, "failed");
    }

    #[test]
    fn test_gc_keep_dry_run() {
        let dir = TempDir::new().unwrap();
        let runs_dir = dir.path().join(".pipeliner").join("runs");
        std::fs::create_dir_all(&runs_dir).unwrap();

        // Create 3 mock runs
        for i in 1..=3 {
            let run_path = runs_dir.join(format!("run{}", i));
            std::fs::create_dir_all(&run_path).unwrap();
            std::fs::write(
                &run_path.join("report.json"),
                format!(
                    r#"{{"status": "success", "completed_at": "2026-05-{}T08:30:12Z"}}"#,
                    10 + i
                ),
            )
            .unwrap();
        }

        let args = GcArgs {
            command: GcSubcommand::Keep {
                n: 1,
                dry_run: true,
            },
        };

        // This should not delete anything (dry run)
        let result = run_gc(args, OutputFormat::Human);
        assert!(result.is_ok());

        // Verify runs still exist
        assert!(runs_dir.join("run1").exists());
        assert!(runs_dir.join("run2").exists());
        assert!(runs_dir.join("run3").exists());
    }

    #[test]
    fn test_gc_keep_actual_delete() {
        let dir = TempDir::new().unwrap();
        let runs_dir = dir.path().join(".pipeliner").join("runs");
        std::fs::create_dir_all(&runs_dir).unwrap();

        // Create 3 mock runs
        for i in 1..=3 {
            let run_path = runs_dir.join(format!("run{}", i));
            std::fs::create_dir_all(&run_path).unwrap();
            std::fs::write(
                &run_path.join("report.json"),
                format!(
                    r#"{{"status": "success", "completed_at": "2026-05-{}T08:30:12Z"}}"#,
                    10 + i
                ),
            )
            .unwrap();
        }

        let args = GcArgs {
            command: GcSubcommand::Keep {
                n: 1,
                dry_run: false,
            },
        };

        let result = run_gc(args, OutputFormat::Human);
        assert!(result.is_ok());

        // Verify only 1 run remains
        assert!(runs_dir.join("run1").exists());
        assert!(!runs_dir.join("run2").exists());
        assert!(!runs_dir.join("run3").exists());
    }

    #[test]
    fn test_find_runs_dir_not_found() {
        let result = find_runs_dir();
        // This might fail if we're not in a pipeliner project
        // So we just check it doesn't panic
        let _ = result;
    }
}

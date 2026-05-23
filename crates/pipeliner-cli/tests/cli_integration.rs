use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn create_pipeline_file(dir: &TempDir, content: &str) -> std::path::PathBuf {
    let path = dir.path().join("pipeline.json");
    fs::write(&path, content).unwrap();
    path
}

const VALID_PIPELINE: &str = r#"{
    "name": "test-pipeline",
    "stages": [
        {
            "type": "stage",
            "name": "build",
            "steps": [
                {"type": "echo", "name": "echo-build", "message": "Building"}
            ]
        },
        {
            "type": "stage",
            "name": "test",
            "steps": [
                {"type": "echo", "name": "echo-test", "message": "Testing"}
            ]
        },
        {
            "type": "stage",
            "name": "deploy",
            "steps": [
                {"type": "echo", "name": "echo-deploy", "message": "Deploying"}
            ]
        }
    ]
}"#;

#[test]
fn test_dry_run_flag() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = create_pipeline_file(&dir, VALID_PIPELINE);

    Command::cargo_bin("pipeliner-cli")
        .unwrap()
        .args(["run", path.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains("[DRY-RUN]"));
}

#[test]
fn test_stages_flag() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = create_pipeline_file(&dir, VALID_PIPELINE);

    Command::cargo_bin("pipeliner-cli")
        .unwrap()
        .args(["run", path.to_str().unwrap(), "--stages=build,test"])
        .assert()
        .success();
}

#[test]
fn test_output_json_flag() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = create_pipeline_file(&dir, VALID_PIPELINE);

    Command::cargo_bin("pipeliner-cli")
        .unwrap()
        .args(["--format", "json", "run", path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("pipeline_start"));
}

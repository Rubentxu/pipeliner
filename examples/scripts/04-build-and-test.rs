#!/usr/bin/env rustline-run
//! [dependencies]
//! serde_json = "1.0"

use std::process::Command;

fn main() {
    println!("=== Build Stage ===");

    // Simulate build
    let output = Command::new("echo")
        .arg("Compiling project...")
        .output()
        .expect("Failed to execute echo");
    println!("{}", String::from_utf8_lossy(&output.stdout));

    // Simulate test
    println!("=== Test Stage ===");
    let output = Command::new("echo")
        .arg("Running 42 tests... All passed!")
        .output()
        .expect("Failed to execute echo");
    println!("{}", String::from_utf8_lossy(&output.stdout));

    // Report
    let report = serde_json::json!({
        "build": "success",
        "tests_passed": 42,
        "tests_failed": 0
    });
    println!("=== Report ===");
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
#!/usr/bin/env rustline-run
use std::process::Command;

fn main() {
    println!("Attempting risky operation...");

    let output = Command::new("ls")
        .arg("/nonexistent/directory/that/should/not/exist")
        .output()
        .expect("Failed to execute ls");

    if output.status.success() {
        println!("Operation succeeded!");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Operation failed: {}", stderr.trim());
        std::process::exit(1);
    }
}
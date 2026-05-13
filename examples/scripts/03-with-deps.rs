#!/usr/bin/env rustline-run
//! [dependencies]
//! serde_json = "1.0"

fn main() {
    let data = serde_json::json!({
        "pipeline": "build-pipeline",
        "status": "running",
        "steps": ["compile", "test", "deploy"]
    });
    println!("{}", serde_json::to_string_pretty(&data).unwrap());
}
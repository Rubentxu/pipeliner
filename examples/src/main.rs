//! Simple CI Pipeline - 100% Declarative Jenkins-style
//!
//! Usage: cargo run --example simple-ci
//!
//! This is the most declarative way to write pipelines.
//! No main(), no #[tokio::main], no code outside the DSL!

#[macro_use]
extern crate pipeliner_macros;

pipeline! {
    name = "Simple CI"
    stages {
        stage!("Environment") {
            steps {
                sh!("echo '=== Environment Info ==='")
                sh!("uname -a")
                sh!("rustc --version")
                sh!("cargo --version")
                echo!("Environment ready!")
            }
        }
        
        stage!("Build") {
            steps {
                sh!("cargo build --release")
                echo!("Build complete!")
            }
        }
        
        stage!("Test") {
            steps {
                sh!("cargo test --lib 2>&1 | tail -20")
                echo!("Tests complete!")
            }
        }
    }
}

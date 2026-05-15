//! Pipeline CI: Build + Test + Verify
//!
//! Pipeline completo de CI/CD con el DSL de Pipeliner.
//!
//! ```rust,ignore
//! use pipeliner_macros::pipeline;
//!
//! let my_pipeline = pipeline! {
//!     name = "CI Build Pipeline"
//!     
//!     stages {
//!         stage!("Prerequisites") {
//!             steps {
//!                 sh!("which git")
//!                 sh!("which cargo")
//!                 echo!("Prerequisites verified!")
//!             }
//!         }
//!         
//!         stage!("Build") {
//!             steps {
//!                 sh!("cargo build --release")
//!                 echo!("Build completed!")
//!             }
//!         }
//!         
//!         stage!("Test") {
//!             steps {
//!                 sh!("cargo test --all")
//!                 echo!("All tests passed!")
//!             }
//!         }
//!         
//!         stage!("Verify") {
//!             steps {
//!                 sh!("ls -lh target/release/pipeliner || ls -lh target/debug/pipeliner")
//!                 sh!("stat target/release/pipeliner 2>/dev/null || stat target/debug/pipeliner")
//!                 echo!("Binary verified!")
//!             }
//!         }
//!     }
//! };
//! ```

use pipeliner_macros::pipeline;

fn main() {
    let _pipeline = pipeline! {
        name = "CI Build Pipeline"
        
        stages {
            stage!("Prerequisites") {
                steps {
                    sh!("which git")
                    sh!("which cargo")
                    echo!("Prerequisites verified!")
                }
            }
            
            stage!("Build") {
                steps {
                    sh!("cargo build --release")
                    echo!("Build completed!")
                }
            }
            
            stage!("Test") {
                steps {
                    sh!("cargo test --all")
                    echo!("All tests passed!")
                }
            }
            
            stage!("Verify") {
                steps {
                    sh!("ls -lh target/release/pipeliner || ls -lh target/debug/pipeliner")
                    sh!("stat target/release/pipeliner 2>/dev/null || stat target/debug/pipeliner")
                    echo!("Binary verified!")
                }
            }
        }
    };
    
    println!("Pipeline defined: CI Build Pipeline");
}

//! Procedural macros for Pipeliner DSL.
//!
//! This crate provides custom derive macros and procedural macros
//! for defining pipelines in a more ergonomic way.
//!
//! ## Available Macros
//!
//! - `pipeline!`: Define a pipeline (returns Pipeline)
//! - `run!`: Execute a pipeline immediately
//!
//! ## Simplified DSL Usage
//!
//! ```rust,ignore
//! use pipeliner_core::prelude::*;
//!
//! let pipeline = pipeline! {
//!     agent { docker("rust:latest") }
//!     stages {
//!         stage!("Build", steps!(
//!             sh!("cargo build")
//!         ))
//!     }
//! };
//!
//! run!(pipeline);  // Execute immediately!
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, parse_macro_input};

/// Execute a pipeline immediately with LocalExecutor.
///
/// # Example
///
/// ```rust,ignore
/// use pipeliner_core::prelude::*;
/// use pipeliner_macros::run;
///
/// let pipeline = pipeline! {
///     agent { any() }
///     stages { stage!("Test", steps!(sh!("cargo test"))) }
/// };
///
/// run!(pipeline);
/// ```
#[proc_macro]
pub fn run(input: TokenStream) -> TokenStream {
    let pipeline = parse_macro_input!(input as Expr);

    let expanded = quote! {
        {
            use pipeliner_executor::LocalExecutor;
            let executor = LocalExecutor::new();
            let result = executor.execute(&#pipeline);
            eprintln!("[PIPELINE] Executed with {} step results", result.len());
            result
        }
    };

    TokenStream::from(expanded)
}

/// Execute a pipeline with custom executor.
///
/// # Example
///
/// ```rust,ignore
/// use pipeliner_core::prelude::*;
/// use pipeliner_executor::DockerExecutor;
/// use pipeliner_macros::run_with;
///
/// let pipeline = pipeline! { ... };
/// let executor = DockerExecutor::new("rust:latest");
/// run_with!(pipeline, executor);
/// ```
#[proc_macro]
pub fn run_with(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as Expr);
    let inputs: Vec<Expr> = vec![];
    let expanded = quote! {};

    TokenStream::from(expanded)
}

/// Execute a pipeline in debug mode with verbose output.
#[proc_macro]
pub fn debug_run(input: TokenStream) -> TokenStream {
    let pipeline = parse_macro_input!(input as Expr);

    let expanded = quote! {
        {
            use pipeliner_executor::LocalExecutor;
            use pipeliner_core::Pipeline;

            eprintln!("[DEBUG] Pipeline name: {:?}", #pipeline.name());
            eprintln!("[DEBUG] Stages: {}", #pipeline.stages.len());

            for (i, stage) in #pipeline.stages.iter().enumerate() {
                eprintln!("[DEBUG]   Stage {}: {} ({} steps)",
                    i + 1, stage.name, stage.steps.len());
            }

            let executor = LocalExecutor::new();
            let result = executor.execute(&#pipeline);
            eprintln!("[DEBUG] Result: {} steps executed", result.len());
            result
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for Pipeline trait.
#[proc_macro_derive(Pipeline)]
pub fn pipeline_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl pipeliner_core::Pipeline for #name {
            fn name(&self) -> Option<&str> {
                self.name.as_deref()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for Stage trait.
#[proc_macro_derive(Stage)]
pub fn stage_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl pipeliner_core::Stage for #name {
            fn name(&self) -> &str {
                &self.name
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for Step trait.
#[proc_macro_derive(Step)]
pub fn step_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl pipeliner_core::Step for #name {
            fn step_type(&self) -> &pipeliner_core::StepType {
                &self.step_type
            }
        }
    };

    TokenStream::from(expanded)
}

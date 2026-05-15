//! Procedural macros for Pipeliner DSL.
//!
//! This crate provides procedural macros for defining pipelines
//! in a Jenkinsfile-style DSL.
//!
//! ## Available Macros
//!
//! ### Step macros
//! - `sh!("command")` - Shell command step
//! - `echo!("message")` - Echo message step
//! - `agent!("model") { ... }` - LLM agent step
//!
//! ### Stage macros
//! - `stage!("name", vec![...steps...])` - Stage with steps
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pipeliner_core::{Pipeline, Stage, Step, LlmAgentConfig};
//! use pipeliner_macros::{sh, echo, stage};
//!
//! let pipeline = Pipeline::new()
//!     .with_name(\"My Pipeline\")
//!     .with_stage(stage!(\"Build\", vec![
//!         sh!(\"cargo build\"),
//!         echo!(\"Build complete!\"),
//!     ]));
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

/// Create a shell command step
///
/// # Example
/// ```rust,ignore
/// sh!("cargo build --release")
/// ```
#[proc_macro]
pub fn sh(input: TokenStream) -> TokenStream {
    let command: Expr = syn::parse(input).expect("Expected a string literal");

    quote! {
        pipeliner_core::Step::shell(#command)
    }
    .into()
}

/// Create an echo message step
///
/// # Example
/// ```rust,ignore
/// echo!("Hello, world!")
/// ```
#[proc_macro]
pub fn echo(input: TokenStream) -> TokenStream {
    let message: Expr = syn::parse(input).expect("Expected a string literal");

    quote! {
        pipeliner_core::Step::echo(#message)
    }
    .into()
}

/// Create a stage with steps
///
/// # Example
/// ```rust,ignore
/// stage!("Build", vec![sh!("cargo build"), echo!("Done!")])
/// ```
#[proc_macro]
pub fn stage(input: TokenStream) -> TokenStream {
    // Parse: "name", [steps...]
    let input_expr: Expr = syn::parse(input).expect("Expected (name, [steps...])");
    
    match input_expr {
        Expr::Tuple(tuple) => {
            if tuple.elems.len() != 2 {
                return syn::Error::new_spanned(
                    &tuple,
                    "stage! expects format: stage!(\"name\", [steps...])"
                ).to_compile_error().into();
            }
            
            let name = &tuple.elems[0];
            let steps = &tuple.elems[1];
            
            quote! {
                pipeliner_core::Stage::new(#name)
                    .with_steps(#steps)
            }.into()
        }
        _ => {
            syn::Error::new_spanned(
                &input_expr,
                "stage! expects format: stage!(\"name\", [steps...])"
            ).to_compile_error().into()
        }
    }
}

/// Create an agent step with configuration
///
/// # Example
/// ```rust,ignore
/// agent!("claude-3-5-sonnet", LlmAgentConfig::new("claude").with_prompt("Hello"))
/// ```
#[proc_macro]
pub fn agent(input: TokenStream) -> TokenStream {
    // Parse: "model", config_expr
    let input_expr: Expr = syn::parse(input).expect("Expected (model, config)");
    
    match input_expr {
        Expr::Tuple(tuple) => {
            if tuple.elems.len() != 2 {
                return syn::Error::new_spanned(
                    &tuple,
                    "agent! expects format: agent!(\"model\", config)"
                ).to_compile_error().into();
            }
            
            let model = &tuple.elems[0];
            let config = &tuple.elems[1];
            
            quote! {
                pipeliner_core::Step::agent(#config.with_model(#model))
            }.into()
        }
        _ => {
            syn::Error::new_spanned(
                &input_expr,
                "agent! expects format: agent!(\"model\", config)"
            ).to_compile_error().into()
        }
    }
}

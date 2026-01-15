//! Procedural macros for Pipeliner DSL.
//!
//! This crate provides custom derive macros and procedural macros
//! for defining pipelines in a more ergonomic way.
//!
//! ## Available Macros
//!
//! - `#[pipeline]`: Derive macro for structs that represent pipelines
//!
//! ## Example
//!
//! ```rust,ignore
//! use pipeliner_macros::pipeline;
//!
//! #[pipeline]
//! struct MyPipeline {
//!     name: String,
//!     stages: Vec<Stage>,
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// Derive macro for pipeline types.
///
/// This macro generates the implementation of `Pipeline` trait
/// and provides serialization support.
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

/// Derive macro for stage types.
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

/// Derive macro for step types.
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

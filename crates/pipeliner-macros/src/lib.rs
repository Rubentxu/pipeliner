//! Procedural macros for Pipeliner DSL.
//!
//! This crate provides the `pipeline!` macro for defining pipelines
//! in a Jenkinsfile-style declarative syntax.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pipeliner_core::{Pipeline, Stage, PipelineRunner};
//! use pipeliner_macros::pipeline;
//!
//! let pl = pipeline! {
//!     name = "CI Pipeline"
//!     
//!     stages {
//!         stage!("Build") {
//!             steps {
//!                 sh!("cargo build --release")
//!                 echo!("Build complete!")
//!             }
//!         }
//!     }
//! };
//! ```

use proc_macro::TokenStream;
use quote::quote;

/// Pipeline definition
#[derive(Default)]
struct PipelineDef {
    name: Option<String>,
    stages: Vec<StageDef>,
}

/// Stage definition
struct StageDef {
    name: String,
    steps: Vec<StepDef>,
}

/// Step definition
enum StepDef {
    Shell(String),
    Echo(String),
}

/// Create a declarative pipeline
///
/// # Example
/// ```rust,ignore
/// pipeline! {
///     name = "My Pipeline"
///     stages {
///         stage!("Build") {
///             steps {
///                 sh!("cargo build")
///                 echo!("Build done!")
///             }
///         }
///     }
/// }
/// ```
#[proc_macro]
pub fn pipeline(input: TokenStream) -> TokenStream {
    let pipeline = match parse_pipeline(input) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };
    
    // Generate the pipeline struct
    let name = pipeline.name.as_ref()
        .map(|n| quote! { .with_name(#n) })
        .unwrap_or_else(|| quote! {});
    
    // Build stage expressions
    let stage_exprs: Vec<_> = pipeline.stages.iter().map(|s| {
        let stage_name = &s.name;
        
        // Generate steps
        let step_exprs: Vec<_> = s.steps.iter().map(|step| {
            match step {
                StepDef::Shell(cmd) => quote! {
                    Step::shell(#cmd)
                },
                StepDef::Echo(msg) => quote! {
                    Step::echo(#msg)
                },
            }
        }).collect();
        
        quote! {
            Stage::new(#stage_name)
                .with_steps(vec![#(#step_exprs),*])
        }
    }).collect();
    
    // Chain with_stage() calls
    let mut pipeline_expr = quote! { Pipeline::new() #name };
    for stage_expr in &stage_exprs {
        pipeline_expr = quote! { #pipeline_expr .with_stage(#stage_expr) };
    }
    
    pipeline_expr.into()
}

fn parse_pipeline(input: TokenStream) -> Result<PipelineDef, syn::Error> {
    let mut pipeline = PipelineDef::default();
    let input: proc_macro2::TokenStream = input.into();
    let tokens: Vec<_> = input.into_iter().collect();
    let mut i = 0;
    
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(ident) = &tokens[i] {
            match ident.to_string().as_str() {
                "name" => {
                    i += 1;
                    // Skip = punct if present
                    if let Some(proc_macro2::TokenTree::Punct(_)) = tokens.get(i) {
                        i += 1;
                    }
                    // Get literal
                    if let Some(proc_macro2::TokenTree::Literal(lit)) = tokens.get(i) {
                        let name = lit.to_string().trim_matches('"').to_string();
                        pipeline.name = Some(name);
                    }
                    i += 1;
                }
                "stages" => {
                    i += 1;
                    // Get the brace group with stages
                    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                        if g.delimiter() == proc_macro2::Delimiter::Brace {
                            let inner_tokens: Vec<_> = g.stream().into_iter().collect();
                            let (stages, _) = parse_stages_from_tokens(&inner_tokens);
                            pipeline.stages = stages;
                        }
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        } else {
            i += 1;
        }
    }
    
    Ok(pipeline)
}

fn parse_stages_from_tokens(tokens: &[proc_macro2::TokenTree]) -> (Vec<StageDef>, usize) {
    let mut stages = Vec::new();
    let mut i = 0;
    
    while i < tokens.len() {
        // Look for stage! macro invocations
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            if id.to_string() == "stage" && i + 1 < tokens.len() {
                if let proc_macro2::TokenTree::Punct(p) = &tokens[i + 1] {
                    if p.to_string() == "!" {
                        i += 2; // skip stage!
                        
                        // Parse stage name from group
                        let name = if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                            if g.delimiter() == proc_macro2::Delimiter::Parenthesis {
                                let inner = g.to_string();
                                inner.trim_matches(|c| c == '(' || c == ')').trim_matches('"').to_string()
                            } else {
                                i += 1;
                                continue;
                            }
                        } else {
                            i += 1;
                            continue;
                        };
                        i += 1; // skip name group
                        
                        // Parse stage body from brace group
                        let steps = if let Some(proc_macro2::TokenTree::Group(body)) = tokens.get(i) {
                            if body.delimiter() == proc_macro2::Delimiter::Brace {
                                parse_steps_from_tokens(&body)
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };
                        i += 1;
                        
                        stages.push(StageDef { name, steps });
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    
    (stages, i)
}

/// Parse steps from a stage body - handles sh! and echo! DSL syntax
fn parse_steps_from_tokens(body: &proc_macro2::Group) -> Vec<StepDef> {
    let body_tokens: Vec<_> = body.stream().into_iter().collect();
    let mut i = 0;
    let mut steps = Vec::new();
    
    // Find the steps block
    while i < body_tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &body_tokens[i] {
            if id.to_string() == "steps" {
                i += 1;
                // Get the brace group with steps
                if let proc_macro2::TokenTree::Group(g) = &body_tokens[i] {
                    if g.delimiter() == proc_macro2::Delimiter::Brace {
                        let step_tokens: Vec<_> = g.stream().into_iter().collect();
                        
                        // Parse each step
                        let mut j = 0;
                        while j < step_tokens.len() {
                            // Look for sh! or echo! invocations
                            if let proc_macro2::TokenTree::Ident(id) = &step_tokens[j] {
                                let macro_name = id.to_string();
                                
                                if (macro_name == "sh" || macro_name == "echo") 
                                    && j + 2 < step_tokens.len() {
                                    
                                    // Check for ! 
                                    if let proc_macro2::TokenTree::Punct(p) = &step_tokens[j + 1] {
                                        if p.to_string() == "!" {
                                            // Found macro invocation
                                            // Get the argument group
                                            if let proc_macro2::TokenTree::Group(arg_group) = &step_tokens[j + 2] {
                                                // Extract the string argument
                                                let arg_str = arg_group.to_string();
                                                let arg = arg_str.trim_matches('"');
                                                
                                                match macro_name.as_str() {
                                                    "sh" => steps.push(StepDef::Shell(arg.to_string())),
                                                    "echo" => steps.push(StepDef::Echo(arg.to_string())),
                                                    _ => {}
                                                }
                                                
                                                j += 3;
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                            j += 1;
                        }
                    }
                }
                break;
            }
        }
        i += 1;
    }
    
    steps
}

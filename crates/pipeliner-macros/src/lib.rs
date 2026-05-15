//! Procedural macros for Pipeliner DSL.

use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

/// Pipeline definition
#[derive(Default)]
struct PipelineDef {
    name: Option<String>,
    stages: Vec<StageDef>,
}

/// Stage definition - stores raw tokens for steps
struct StageDef {
    name: String,
    steps_tokens: Vec<proc_macro2::TokenStream>,
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
    let mut stage_exprs = Vec::new();
    for s in &pipeline.stages {
        let stage_name = &s.name;
        
        // Generate each step from raw tokens
        let step_exprs: Vec<proc_macro2::TokenStream> = s.steps_tokens.iter()
            .map(|tokens| {
                tokens.clone()
            })
            .collect();
        
        stage_exprs.push(quote! {
            Stage::new(#stage_name)
                .with_steps(vec![#(#step_exprs),*])
        });
    }
    
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
                        let steps_tokens = if let Some(proc_macro2::TokenTree::Group(body)) = tokens.get(i) {
                            if body.delimiter() == proc_macro2::Delimiter::Brace {
                                parse_steps_tokens(&body)
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };
                        i += 1;
                        
                        stages.push(StageDef { name, steps_tokens });
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    
    (stages, i)
}

fn parse_steps_tokens(body: &proc_macro2::Group) -> Vec<proc_macro2::TokenStream> {
    let body_tokens: Vec<_> = body.stream().into_iter().collect();
    let mut i = 0;
    let mut steps = Vec::new();
    
    while i < body_tokens.len() {
        // Look for steps keyword
        if let proc_macro2::TokenTree::Ident(id) = &body_tokens[i] {
            if id.to_string() == "steps" {
                i += 1;
                // Get the brace group with steps
                if let Some(proc_macro2::TokenTree::Group(g)) = body_tokens.get(i) {
                    if g.delimiter() == proc_macro2::Delimiter::Brace {
                        // Collect all tokens inside steps block
                        let step_tokens: Vec<_> = g.stream().into_iter().collect();
                        
                        // Group tokens by macro invocations (sh!, echo!, etc.)
                        let mut j = 0;
                        while j < step_tokens.len() {
                            // Check if this is a macro invocation (Ident followed by !)
                            if let proc_macro2::TokenTree::Ident(id) = &step_tokens[j] {
                                if j + 1 < step_tokens.len() {
                                    if let proc_macro2::TokenTree::Punct(p) = &step_tokens[j + 1] {
                                        if p.to_string() == "!" {
                                            // Found macro invocation, collect whole thing
                                            // Find matching group
                                            if let Some(proc_macro2::TokenTree::Group(_)) = step_tokens.get(j + 2) {
                                                // Collect macro name + ! + group
                                                let mut macro_tokens = Vec::new();
                                                macro_tokens.push(step_tokens[j].clone());
                                                macro_tokens.push(step_tokens[j + 1].clone());
                                                macro_tokens.push(step_tokens[j + 2].clone());
                                                steps.push(macro_tokens.into_iter().collect());
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

// ============================================================================
// Step Macros (sh, echo)
// ============================================================================

/// Create a shell command step
#[proc_macro]
pub fn sh(input: TokenStream) -> TokenStream {
    let command: Expr = syn::parse(input).expect("Expected a string literal");
    quote! {
        pipeliner_core::Step::shell(#command)
    }
    .into()
}

/// Create an echo message step
#[proc_macro]
pub fn echo(input: TokenStream) -> TokenStream {
    let message: Expr = syn::parse(input).expect("Expected a string literal");
    quote! {
        pipeliner_core::Step::echo(#message)
    }
    .into()
}

// ============================================================================
// Stage Macro
// ============================================================================

/// Create a stage with steps
#[proc_macro]
pub fn stage(input: TokenStream) -> TokenStream {
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

// ============================================================================
// Agent Macro
// ============================================================================

/// Create an agent step with configuration
#[proc_macro]
pub fn agent(input: TokenStream) -> TokenStream {
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

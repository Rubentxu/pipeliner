//! Pipeliner DSL - Jenkins-style declarative pipelines
//!
//! # 100% Declarative (like Jenkins Jenkinsfile)
//!
//! ```ignore
//! pipeline! {
//!     name = "CI"
//!     stages {
//!         stage!("Build") {
//!             steps {
//!                 sh!("cargo build")
//!             }
//!         }
//!         stage!("Test") {
//!             parallel {
//!                 stage!("Linux") { steps { sh!("test linux") } }
//!                 stage!("Windows") { steps { sh!("test windows") } }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! That's it! No main(), no #[tokio::main], no code outside the DSL.

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
    content: StageContent,
}

/// StageContent: sequential steps or parallel stages
enum StageContent {
    Steps(Vec<StepDef>),
    Parallel(Vec<StageDef>),
}

impl Default for StageContent {
    fn default() -> Self { StageContent::Steps(vec![]) }
}

/// Step definition
enum StepDef {
    Shell(String),
    Echo(String),
}

// =============================================================================
// Internal parser for pipeline DSL
// =============================================================================

/// Build pipeline expression (for use in tests and programmatic access)
fn build_pipeline_expr(pipeline: &PipelineDef) -> proc_macro2::TokenStream {
    let name = pipeline.name.as_ref()
        .map(|n| quote! { .with_name(#n) })
        .unwrap_or_else(|| quote! {});
    
    let mut stage_exprs: Vec<_> = vec![];
    
    for s in &pipeline.stages {
        match &s.content {
            StageContent::Steps(steps) => {
                let nm = &s.name;
                let step_exprs: Vec<_> = steps.iter().map(|s| match s {
                    StepDef::Shell(cmd) => quote! { Step::shell(#cmd) },
                    StepDef::Echo(msg) => quote! { Step::echo(#msg) },
                }).collect();
                stage_exprs.push(quote! { Stage::new(#nm).with_steps(vec![#(#step_exprs),*]) });
            }
            StageContent::Parallel(sub) => {
                let sub_exprs: Vec<_> = sub.iter().map(|s| {
                    let sn = &s.name;
                    let steps = match &s.content {
                        StageContent::Steps(steps) => steps.iter().map(|step| match step {
                            StepDef::Shell(cmd) => quote! { Step::shell(#cmd) },
                            StepDef::Echo(msg) => quote! { Step::echo(#msg) },
                        }).collect::<Vec<_>>(),
                        _ => vec![],
                    };
                    quote! { Stage::new(#sn).with_steps(vec![#(#steps),*]) }
                }).collect();
                stage_exprs.push(quote! { Pipeline::parallel(vec![#(#sub_exprs),*]) });
            }
        }
    }
    
    let mut pipeline_expr = quote! { Pipeline::new() #name };
    for se in stage_exprs {
        pipeline_expr = quote! { #pipeline_expr.with_stage(#se) };
    }
    pipeline_expr
}

/// Define a pipeline without executing (for tests)
#[proc_macro]
pub fn pipeline_def(input: TokenStream) -> TokenStream {
    let pipeline = match parse_pipeline_tokens(input.into()) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };
    
    if pipeline.stages.is_empty() {
        return syn::Error::new_spanned(
            &proc_macro2::TokenTree::Group(proc_macro2::Group::new(
                proc_macro2::Delimiter::Brace, 
                proc_macro2::TokenStream::new()
            )), 
            "No stages found in pipeline!"
        ).to_compile_error().into();
    }
    
    build_pipeline_expr(&pipeline).into()
}

fn parse_pipeline_tokens(input: proc_macro2::TokenStream) -> Result<PipelineDef, syn::Error> {
    let tokens: Vec<_> = input.into_iter().collect();
    let mut pipeline = PipelineDef::default();
    let mut i = 0;
    
    while i < tokens.len() {
        match &tokens[i] {
            proc_macro2::TokenTree::Ident(id) => {
                match id.to_string().as_str() {
                    "name" => {
                        i += 1;
                        if matches!(tokens.get(i), Some(proc_macro2::TokenTree::Punct(_))) { i += 1; }
                        if let Some(proc_macro2::TokenTree::Literal(lit)) = tokens.get(i) {
                            pipeline.name = Some(lit.to_string().trim_matches('"').to_string());
                        }
                        i += 1;
                    }
                    "stages" => {
                        i += 1;
                        if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                            if g.delimiter() == proc_macro2::Delimiter::Brace {
                                pipeline.stages = parse_stages(&g);
                            }
                        }
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            _ => i += 1,
        }
    }
    Ok(pipeline)
}

fn parse_stages(g: &proc_macro2::Group) -> Vec<StageDef> {
    let tokens: Vec<_> = g.stream().into_iter().collect();
    let mut stages = Vec::new();
    let mut i = 0;
    
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            if id.to_string() == "stage" {
                if let Some(s) = parse_stage(&tokens, &mut i) {
                    stages.push(s);
                    continue;
                }
            }
        }
        i += 1;
    }
    stages
}

fn parse_stage(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<StageDef> {
    *i += 1; // skip 'stage'
    
    // Skip '!'
    if let Some(proc_macro2::TokenTree::Punct(p)) = tokens.get(*i) {
        if p.to_string() == "!" {
            *i += 1;
        }
    }
    
    // Get name from parentheses
    if let Some(proc_macro2::TokenTree::Group(name_g)) = tokens.get(*i) {
        if name_g.delimiter() == proc_macro2::Delimiter::Parenthesis {
            let name = name_g.to_string().trim_matches(|c| c == '(' || c == ')').trim_matches('"').to_string();
            *i += 1;
            
            // Get body from braces
            if let Some(proc_macro2::TokenTree::Group(body)) = tokens.get(*i) {
                if body.delimiter() == proc_macro2::Delimiter::Brace {
                    *i += 1;
                    return Some(StageDef { name, content: parse_content(&body) });
                }
            }
        }
    }
    None
}

fn parse_content(body: &proc_macro2::Group) -> StageContent {
    let tokens: Vec<_> = body.stream().into_iter().collect();
    let mut i = 0;
    
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            match id.to_string().as_str() {
                "parallel" => return parse_parallel(&tokens, &mut i),
                "steps" => {
                    i += 1;
                    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                        if g.delimiter() == proc_macro2::Delimiter::Brace {
                            return StageContent::Steps(parse_steps(&g));
                        }
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    StageContent::Steps(vec![])
}

fn parse_parallel(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> StageContent {
    *i += 1; // skip 'parallel'
    let mut sub = Vec::new();
    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(*i) {
        if g.delimiter() == proc_macro2::Delimiter::Brace {
            let inner: Vec<_> = g.stream().into_iter().collect();
            let mut j = 0;
            while j < inner.len() {
                if let proc_macro2::TokenTree::Ident(id) = &inner[j] {
                    if id.to_string() == "stage" {
                        j += 1;
                        // Skip !
                        if j < inner.len() {
                            if let proc_macro2::TokenTree::Punct(p) = &inner[j] {
                                if p.to_string() == "!" { j += 1; }
                            }
                        }
                        if let Some(proc_macro2::TokenTree::Group(name_g)) = inner.get(j) {
                            if name_g.delimiter() == proc_macro2::Delimiter::Parenthesis {
                                let name = name_g.to_string().trim_matches(|c| c == '(' || c == ')').trim_matches('"').to_string();
                                j += 1;
                                // Parse body
                                if let Some(proc_macro2::TokenTree::Group(body)) = inner.get(j) {
                                    if body.delimiter() == proc_macro2::Delimiter::Brace {
                                        let content = parse_content(&body);
                                        sub.push(StageDef { name, content });
                                        j += 1;
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
                j += 1;
            }
            *i += 1;
            return StageContent::Parallel(sub);
        }
    }
    *i += 1;
    StageContent::Parallel(vec![])
}

fn parse_steps(g: &proc_macro2::Group) -> Vec<StepDef> {
    let tokens: Vec<_> = g.stream().into_iter().collect();
    let mut steps = Vec::new();
    let mut i = 0;
    
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            let macro_name = id.to_string();
            if (macro_name == "sh" || macro_name == "echo") && i + 2 < tokens.len() {
                if let proc_macro2::TokenTree::Punct(p) = &tokens[i + 1] {
                    if p.to_string() == "!" {
                        if let proc_macro2::TokenTree::Group(args) = &tokens[i + 2] {
                            let arg = args.to_string().trim_matches('"').to_string();
                            match macro_name.as_str() {
                                "sh" => steps.push(StepDef::Shell(arg)),
                                "echo" => steps.push(StepDef::Echo(arg)),
                                _ => {}
                            }
                            i += 3;
                            continue;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    steps
}

// =============================================================================
// pipeline! - 100% Declarative Jenkins-style pipeline
// Generates main() and executes automatically (like Jenkins Jenkinsfile)
// =============================================================================

/// Jenkins-style declarative pipeline
///
/// # Example
///
/// ```ignore
/// pipeline! {
///     name = "CI Pipeline"
///     stages {
///         stage!("Build") {
///             steps {
///                 sh!("cargo build")
///             }
///         }
///         stage!("Test") {
///             parallel {
///                 stage!("Linux") { steps { sh!("test linux") } }
///                 stage!("Windows") { steps { sh!("test windows") } }
///             }
///         }
///     }
/// }
/// ```
///
/// This expands to a complete runnable program with main() and tokio runtime.
#[proc_macro]
pub fn pipeline(input: TokenStream) -> TokenStream {
    let pipeline = match parse_pipeline_tokens(input.into()) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };
    
    if pipeline.stages.is_empty() {
        return syn::Error::new_spanned(
            &proc_macro2::TokenTree::Group(proc_macro2::Group::new(
                proc_macro2::Delimiter::Brace, 
                proc_macro2::TokenStream::new()
            )), 
            "No stages found in pipeline!"
        ).to_compile_error().into();
    }
    
    let pipeline_expr = build_pipeline_expr(&pipeline);
    let pipeline_name = pipeline.name.clone().unwrap_or_else(|| "Unnamed".to_string());
    
    quote! {
        fn main() {
            use pipeliner_core::{Pipeline, Stage, Step, PipelineRunner};
            use tracing_subscriber::EnvFilter;
            
            let _ = tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::from_default_env())
                .try_init();
            
            let mut pipeline = #pipeline_expr;
            
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(async {
                let name = #pipeline_name;
                eprintln!("\n╔══════════════════════════════════════════╗");
                eprintln!("║  {}  ║", format!("{:^38}", name));
                eprintln!("╚══════════════════════════════════════════╝\n");
                
                let mut runner = PipelineRunner::new();
                match runner.run_async(&pipeline).await {
                    Ok(result) => {
                        eprintln!("\n📊 Results:");
                        eprintln!("   Success: {}", if result.success { "✅" } else { "❌" });
                        eprintln!("   Duration: {}ms", result.duration_ms);
                        eprintln!("   Stages: {}", result.stages_executed);
                        eprintln!("   Steps: {}", result.steps_executed);
                        std::process::exit(if result.success { 0 } else { 1 });
                    }
                    Err(e) => {
                        eprintln!("❌ Pipeline failed: {}", e);
                        std::process::exit(1);
                    }
                }
            });
        }
    }.into()
}

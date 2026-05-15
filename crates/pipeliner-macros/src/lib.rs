//! Procedural macros for Pipeliner DSL.

use proc_macro::TokenStream;
use quote::quote;

/// Pipeline definition
#[derive(Default)]
struct PipelineDef {
    name: Option<String>,
    stages: Vec<PipelineItem>,
}

/// An item inside the stages block
enum PipelineItem {
    Stage(StageDef),
    Parallel(Vec<StageDef>),
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
#[proc_macro]
pub fn pipeline(input: TokenStream) -> TokenStream {
    let pipeline = match parse_pipeline(input) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };
    
    let name = pipeline.name.as_ref()
        .map(|n| quote! { .with_name(#n) })
        .unwrap_or_else(|| quote! {});
    
    let stage_exprs: Vec<_> = pipeline.stages.iter().map(|item| {
        match item {
            PipelineItem::Stage(stage) => {
                let stage_name = &stage.name;
                let step_exprs: Vec<_> = stage.steps.iter().map(|step| {
                    match step {
                        StepDef::Shell(cmd) => quote! { Step::shell(#cmd) },
                        StepDef::Echo(msg) => quote! { Step::echo(#msg) },
                    }
                }).collect();
                quote! {
                    Stage::new(#stage_name)
                        .with_steps(vec![#(#step_exprs),*])
                }
            }
            PipelineItem::Parallel(stages) => {
                let parallel_stages: Vec<_> = stages.iter().map(|stage| {
                    let stage_name = &stage.name;
                    let step_exprs: Vec<_> = stage.steps.iter().map(|step| {
                        match step {
                            StepDef::Shell(cmd) => quote! { Step::shell(#cmd) },
                            StepDef::Echo(msg) => quote! { Step::echo(#msg) },
                        }
                    }).collect();
                    quote! {
                        Stage::new(#stage_name)
                            .with_steps(vec![#(#step_exprs),*])
                    }
                }).collect();
                quote! {
                    Pipeline::parallel(vec![#(#parallel_stages),*])
                }
            }
        }
    }).collect();
    
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
                    if let Some(proc_macro2::TokenTree::Punct(_)) = tokens.get(i) {
                        i += 1;
                    }
                    if let Some(proc_macro2::TokenTree::Literal(lit)) = tokens.get(i) {
                        pipeline.name = Some(lit.to_string().trim_matches('"').to_string());
                    }
                    i += 1;
                }
                "stages" => {
                    i += 1;
                    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                        if g.delimiter() == proc_macro2::Delimiter::Brace {
                            let inner: Vec<_> = g.stream().into_iter().collect();
                            pipeline.stages = parse_stages(&inner);
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

fn parse_stages(tokens: &[proc_macro2::TokenTree]) -> Vec<PipelineItem> {
    let mut items = Vec::new();
    let mut i = 0;
    
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            let id_str = id.to_string();
            
            if id_str == "stage" && i + 1 < tokens.len() {
                if let proc_macro2::TokenTree::Punct(p) = &tokens[i + 1] {
                    if p.to_string() == "!" {
                        if let Some(stage) = parse_stage(tokens, &mut i) {
                            items.push(PipelineItem::Stage(stage));
                        }
                        continue;
                    }
                }
            }
            
            if id_str == "parallel" && i + 1 < tokens.len() {
                if let proc_macro2::TokenTree::Punct(p) = &tokens[i + 1] {
                    if p.to_string() == "!" {
                        if let Some(stages) = parse_parallel_group(tokens, &mut i) {
                            items.push(PipelineItem::Parallel(stages));
                        }
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    
    items
}

fn parse_stage(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<StageDef> {
    *i += 2; // skip stage!
    
    let name = if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(*i) {
        if g.delimiter() == proc_macro2::Delimiter::Parenthesis {
            let inner = g.to_string();
            inner.trim_matches(|c| c == '(' || c == ')').trim_matches('"').to_string()
        } else {
            return None;
        }
    } else {
        return None;
    };
    *i += 1;
    
    let steps = if let Some(proc_macro2::TokenTree::Group(body)) = tokens.get(*i) {
        if body.delimiter() == proc_macro2::Delimiter::Brace {
            parse_steps(&body)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    *i += 1;
    
    Some(StageDef { name, steps })
}

fn parse_parallel_group(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<Vec<StageDef>> {
    *i += 2; // skip parallel!
    
    let mut stages = Vec::new();
    
    // Skip optional name
    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(*i) {
        if g.delimiter() == proc_macro2::Delimiter::Parenthesis {
            *i += 1;
        }
    }
    
    if let Some(proc_macro2::TokenTree::Group(body)) = tokens.get(*i) {
        if body.delimiter() == proc_macro2::Delimiter::Brace {
            let body_tokens: Vec<_> = body.stream().into_iter().collect();
            let mut j = 0;
            
            while j < body_tokens.len() {
                if let proc_macro2::TokenTree::Ident(id) = &body_tokens[j] {
                    if id.to_string() == "stage" && j + 1 < body_tokens.len() {
                        if let proc_macro2::TokenTree::Punct(p) = &body_tokens[j + 1] {
                            if p.to_string() == "!" {
                                if let Some(stage) = parse_stage(&body_tokens, &mut j) {
                                    stages.push(stage);
                                }
                                continue;
                            }
                        }
                    }
                }
                j += 1;
            }
        }
    }
    *i += 1;
    
    if stages.is_empty() { None } else { Some(stages) }
}

fn parse_steps(body: &proc_macro2::Group) -> Vec<StepDef> {
    let tokens: Vec<_> = body.stream().into_iter().collect();
    let mut steps = Vec::new();
    let mut i = 0;
    
    // Find steps block and parse
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            if id.to_string() == "steps" {
                i += 1;
                // If it's a Group with brace
                if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                    if g.delimiter() == proc_macro2::Delimiter::Brace {
                        let inner_steps = parse_steps_from_group(&g);
                        steps.extend(inner_steps);
                        i += 1;
                        continue;
                    }
                }
            }
            
            let macro_name = id.to_string();
            if (macro_name == "sh" || macro_name == "echo") && i + 2 < tokens.len() {
                if let proc_macro2::TokenTree::Punct(p) = &tokens[i + 1] {
                    if p.to_string() == "!" {
                        if let proc_macro2::TokenTree::Group(arg) = &tokens[i + 2] {
                            let arg_str = arg.to_string().trim_matches('"').to_string();
                            match macro_name.as_str() {
                                "sh" => steps.push(StepDef::Shell(arg_str)),
                                "echo" => steps.push(StepDef::Echo(arg_str)),
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

fn parse_steps_from_group(group: &proc_macro2::Group) -> Vec<StepDef> {
    let tokens: Vec<_> = group.stream().into_iter().collect();
    let mut steps = Vec::new();
    let mut i = 0;
    
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            let macro_name = id.to_string();
            
            if (macro_name == "sh" || macro_name == "echo") && i + 2 < tokens.len() {
                if let proc_macro2::TokenTree::Punct(p) = &tokens[i + 1] {
                    if p.to_string() == "!" {
                        if let proc_macro2::TokenTree::Group(arg) = &tokens[i + 2] {
                            let arg_str = arg.to_string().trim_matches('"').to_string();
                            match macro_name.as_str() {
                                "sh" => steps.push(StepDef::Shell(arg_str)),
                                "echo" => steps.push(StepDef::Echo(arg_str)),
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

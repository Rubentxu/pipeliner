//! Pipeline parsing module for the NEW syntax
//!
//! Syntax: stage "Name" { steps { sh "cmd"; } }

use quote::quote;

// =============================================================================
// Data Structures for Parsing (PipelineSpec-compatible)
// =============================================================================

#[derive(Default, Debug, PartialEq, Eq)]
pub struct PipelineDef {
    pub env: Option<Vec<(String, String)>>,
    pub stages: Vec<StageDef>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StageDef {
    pub name: String,
    pub env: Option<Vec<(String, String)>>,
    pub options: Option<OptionsDef>,
    pub content: StageContent,
}

#[derive(Debug, PartialEq, Eq)]
pub struct OptionsDef {
    pub timeout_secs: Option<u64>,
    pub retry: Option<u32>,
}

impl Default for OptionsDef {
    fn default() -> Self {
        OptionsDef { timeout_secs: None, retry: None }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StageContent {
    Steps(Vec<StepDef>),
    Parallel(Vec<StageDef>),
}

impl Default for StageContent {
    fn default() -> Self { StageContent::Steps(vec![]) }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum StepDef {
    Shell(ShellDef),
    Echo(String),
    Dir(DirDef),
    WithEnv(WithEnvDef),
    LetOutput(LetOutputDef),
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(missing_copy_implementations)]
pub struct ShellDef {
    pub script: String,
    pub kind: Option<String>,
    pub label: Option<String>,
    pub interpolation: Option<String>,
    pub capture_stdout: bool,
    pub return_status: bool,
    pub fail_on_nonzero: bool,
}

impl ShellDef {
    pub fn new(script: &str) -> Self {
        ShellDef {
            script: script.to_string(),
            kind: None,
            label: None,
            interpolation: None,
            capture_stdout: false,
            return_status: false,
            fail_on_nonzero: true,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DirDef {
    pub path: String,
    pub steps: Vec<StepDef>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct WithEnvDef {
    pub env: Vec<(String, String)>,
    pub steps: Vec<StepDef>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LetOutputDef {
    pub var_name: String,
    pub inner: Box<StepDef>,
}

// =============================================================================
// Token Parsing Utilities
// =============================================================================

fn parse_string_literal(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<String> {
    if let proc_macro2::TokenTree::Literal(lit) = &tokens[*i] {
        let s = lit.to_string();
        if s.starts_with('"') && s.ends_with('"') {
            let content = &s[1..s.len()-1];
            *i += 1;
            return Some(content.to_string());
        }
    }
    None
}

fn parse_block<F, T>(tokens: &[proc_macro2::TokenTree], i: &mut usize, f: F) -> Option<T>
where
    F: Fn(&[proc_macro2::TokenTree]) -> T,
{
    if let proc_macro2::TokenTree::Group(g) = &tokens[*i] {
        if g.delimiter() == proc_macro2::Delimiter::Brace {
            *i += 1;
            return Some(f(&g.stream().into_iter().collect::<Vec<_>>()));
        }
    }
    None
}

fn skip_semicolons(tokens: &[proc_macro2::TokenTree], i: &mut usize) {
    while let Some(proc_macro2::TokenTree::Punct(p)) = tokens.get(*i) {
        if p.to_string() == ";" {
            *i += 1;
        } else {
            break;
        }
    }
}

// =============================================================================
// Pipeline Parsing - NEW SYNTAX
// =============================================================================

pub fn parse_pipeline(tokens: Vec<proc_macro2::TokenTree>) -> PipelineDef {
    let mut pipeline = PipelineDef::default();
    let mut i = 0;

    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            match id.to_string().as_str() {
                "env" => {
                    if let Some(env) = parse_env_block(&tokens, &mut i) {
                        pipeline.env = Some(env);
                    }
                }
                "stages" => {
                    if let Some(stgs) = parse_block(&tokens, &mut i, parse_stages) {
                        pipeline.stages = stgs;
                    }
                }
                _ => {
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }
    pipeline
}

fn parse_env_block(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<Vec<(String, String)>> {
    *i += 1; // skip 'env'
    parse_block(tokens, i, |tokens| {
        parse_env_content(tokens)
    })
}

fn parse_env_content(tokens: &[proc_macro2::TokenTree]) -> Vec<(String, String)> {
    let mut env_vars = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            let key = id.to_string();
            i += 1;

            // Expect '='
            if let Some(proc_macro2::TokenTree::Punct(p)) = tokens.get(i) {
                if p.to_string() == "=" {
                    i += 1;
                    if let Some(value) = parse_string_literal(tokens, &mut i) {
                        env_vars.push((key, value));
                    }
                }
            }
            skip_semicolons(tokens, &mut i);
        } else {
            i += 1;
        }
    }
    env_vars
}

pub fn parse_stages(tokens: &[proc_macro2::TokenTree]) -> Vec<StageDef> {
    let mut stages = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            if id.to_string() == "stage" {
                if let Some(s) = parse_stage(tokens, &mut i) {
                    stages.push(s);
                    continue;
                }
            }
        }
        i += 1;
    }
    stages
}

/// Parse: stage "Name" { body }
pub fn parse_stage(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<StageDef> {
    *i += 1;
    let name = parse_string_literal(tokens, i)?;
    if let Some(proc_macro2::TokenTree::Group(body)) = tokens.get(*i) {
        if body.delimiter() == proc_macro2::Delimiter::Brace {
            *i += 1;
            let body_tokens: Vec<_> = body.stream().into_iter().collect();
            return Some(parse_stage_body(&name, &body_tokens));
        }
    }
    None
}

fn parse_stage_body(name: &str, tokens: &[proc_macro2::TokenTree]) -> StageDef {
    let mut stage = StageDef {
        name: name.to_string(),
        env: None,
        options: None,
        content: StageContent::Steps(vec![]),
    };

    let mut i = 0;
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            match id.to_string().as_str() {
                "env" => {
                    if let Some(env) = parse_env_block(tokens, &mut i) {
                        stage.env = Some(env);
                    }
                }
                "options" => {
                    if let Some(opts) = parse_options_block(tokens, &mut i) {
                        stage.options = Some(opts);
                    }
                }
                "steps" => {
                    i += 1; // Skip "steps" keyword
                    if let Some(steps) = parse_block(tokens, &mut i, parse_steps) {
                        stage.content = StageContent::Steps(steps);
                        skip_semicolons(tokens, &mut i);
                        continue;
                    }
                }
                "parallel" => {
                    i += 1; // Skip "parallel" keyword
                    if let Some(parallel) = parse_block(tokens, &mut i, parse_parallel) {
                        stage.content = StageContent::Parallel(parallel);
                        continue;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    stage
}

fn parse_options_block(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<OptionsDef> {
    *i += 1; // skip 'options'
    parse_block(tokens, i, |tokens| {
        parse_options_content(tokens)
    })
}

fn parse_options_content(tokens: &[proc_macro2::TokenTree]) -> OptionsDef {
    let mut opts = OptionsDef::default();
    let mut i = 0;

    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            match id.to_string().as_str() {
                "timeout" => {
                    i += 1;
                    // Parse timeout value (could be minutes(n) or raw number)
                    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                        if g.delimiter() == proc_macro2::Delimiter::Parenthesis {
                            i += 1;
                            let inner: Vec<_> = g.stream().into_iter().collect();
                            let mut j = 0;
                            // Check if first token is "minutes"
                            if let Some(proc_macro2::TokenTree::Ident(first_ident)) = inner.get(j) {
                                if first_ident.to_string() == "minutes" {
                                    j += 1;
                                    // Next should be (n) group
                                    if let Some(proc_macro2::TokenTree::Group(inner_g)) = inner.get(j) {
                                        if inner_g.delimiter() == proc_macro2::Delimiter::Parenthesis {
                                            let inner_inner: Vec<_> = inner_g.stream().into_iter().collect();
                                            let mut k = 0;
                                            if let Some(proc_macro2::TokenTree::Literal(lit)) = inner_inner.get(k) {
                                                let num_str = lit.to_string();
                                                if let Ok(mins) = num_str.parse::<u64>() {
                                                    opts.timeout_secs = Some(mins * 60);
                                                }
                                            }
                                        }
                                    }
                                    continue;
                                }
                            }
                            // Otherwise, try to parse as raw number
                            if let Some(tt) = parse_ident_or_number(&inner, &mut j) {
                                if let Ok(secs) = tt.parse::<u64>() {
                                    opts.timeout_secs = Some(secs);
                                }
                            }
                        }
                    }
                }
                "retry" => {
                    i += 1;
                    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                        if g.delimiter() == proc_macro2::Delimiter::Parenthesis {
                            i += 1;
                            let inner: Vec<_> = g.stream().into_iter().collect();
                            let mut j = 0;
                            if let Some(val) = parse_ident_or_number(&inner, &mut j) {
                                if let Ok(retry_val) = val.parse::<u32>() {
                                    opts.retry = Some(retry_val);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    opts
}

fn parse_ident_or_number(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<String> {
    if let Some(t) = tokens.get(*i) {
        let s = match t {
            proc_macro2::TokenTree::Ident(id) => id.to_string(),
            proc_macro2::TokenTree::Literal(lit) => lit.to_string(),
            _ => return None,
        };
        *i += 1;
        return Some(s);
    }
    None
}

fn parse_parallel(tokens: &[proc_macro2::TokenTree]) -> Vec<StageDef> {
    let mut stages = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            if id.to_string() == "stage" {
                if let Some(s) = parse_stage(tokens, &mut i) {
                    stages.push(s);
                    continue;
                }
            }
        }
        i += 1;
    }
    stages
}

pub fn parse_steps(tokens: &[proc_macro2::TokenTree]) -> Vec<StepDef> {
    let mut steps = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            match id.to_string().as_str() {
                "sh" | "echo" | "dir" | "with_env" | "let_output" => {
                    if let Some(step) = parse_step(tokens, &mut i) {
                        steps.push(step);
                        skip_semicolons(tokens, &mut i);
                        continue;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    steps
}

fn parse_step(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<StepDef> {
    let step_name = if let proc_macro2::TokenTree::Ident(id) = &tokens[*i] {
        id.to_string()
    } else {
        return None;
    };
    *i += 1;

    match step_name.as_str() {
        "sh" => parse_sh_step(tokens, i),
        "echo" => parse_echo_step(tokens, i),
        "dir" => parse_dir_step(tokens, i),
        "with_env" => parse_with_env_step(tokens, i),
        "let_output" => parse_let_output_step(tokens, i),
        _ => None,
    }
}

fn parse_sh_step(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<StepDef> {
    // Check if it's a block-based sh { } with all options
    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(*i) {
        if g.delimiter() == proc_macro2::Delimiter::Brace {
            *i += 1;
            let inner: Vec<_> = g.stream().into_iter().collect();
            return Some(StepDef::Shell(parse_sh_block(&inner)));
        }
    }

    // Check for "raw" modifier: sh raw "..."
    if let Some(proc_macro2::TokenTree::Ident(id)) = tokens.get(*i) {
        if id.to_string() == "raw" {
            *i += 1;
            if let Some(script) = parse_string_literal(tokens, i) {
                let mut shell = ShellDef::new(&script);
                shell.interpolation = Some("raw".to_string());
                return Some(StepDef::Shell(shell));
            }
        }
    }

    // Simple sh "script"
    if let Some(script) = parse_string_literal(tokens, i) {
        return Some(StepDef::Shell(ShellDef::new(&script)));
    }

    None
}

fn parse_sh_block(tokens: &[proc_macro2::TokenTree]) -> ShellDef {
    let mut shell = ShellDef::new("");
    let mut i = 0;

    while i < tokens.len() {
        if let proc_macro2::TokenTree::Ident(id) = &tokens[i] {
            match id.to_string().as_str() {
                "script" => {
                    i += 1;
                    if let Some(s) = parse_string_literal(tokens, &mut i) {
                        shell.script = s;
                    }
                }
                "kind" => {
                    i += 1;
                    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                        if g.delimiter() == proc_macro2::Delimiter::Parenthesis {
                            i += 1;
                            let inner: Vec<_> = g.stream().into_iter().collect();
                            let mut j = 0;
                            if let Some(kind_val) = parse_ident_or_number(&inner, &mut j) {
                                shell.kind = Some(kind_val);
                            }
                        }
                    }
                }
                "label" => {
                    i += 1;
                    if let Some(l) = parse_string_literal(tokens, &mut i) {
                        shell.label = Some(l);
                    }
                }
                "interpolation" => {
                    i += 1;
                    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i) {
                        if g.delimiter() == proc_macro2::Delimiter::Parenthesis {
                            i += 1;
                            let inner: Vec<_> = g.stream().into_iter().collect();
                            let mut j = 0;
                            if let Some(mode) = parse_ident_or_number(&inner, &mut j) {
                                shell.interpolation = Some(mode);
                            }
                        }
                    }
                }
                "capture_stdout" => {
                    shell.capture_stdout = true;
                    i += 1;
                }
                "return_status" => {
                    shell.return_status = true;
                    i += 1;
                }
                "allow_failure" => {
                    shell.fail_on_nonzero = false;
                    i += 1;
                }
                _ => {}
            }
        } else {
            i += 1;
        }
    }
    shell
}

fn parse_echo_step(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<StepDef> {
    if let Some(msg) = parse_string_literal(tokens, i) {
        Some(StepDef::Echo(msg))
    } else {
        None
    }
}

fn parse_dir_step(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<StepDef> {
    // Parse: dir "path" { steps... }
    let path = parse_string_literal(tokens, i)?;

    if let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(*i) {
        if g.delimiter() == proc_macro2::Delimiter::Brace {
            *i += 1;
            let inner: Vec<_> = g.stream().into_iter().collect();
            let steps = parse_steps(&inner);
            return Some(StepDef::Dir(DirDef { path, steps }));
        }
    }
    None
}

fn parse_with_env_step(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<StepDef> {
    // Parse: with_env { env vars } { steps }
    if let Some(proc_macro2::TokenTree::Group(env_g)) = tokens.get(*i) {
        if env_g.delimiter() == proc_macro2::Delimiter::Brace {
            *i += 1;
            let env_inner: Vec<_> = env_g.stream().into_iter().collect();
            let env_vars = parse_env_content(&env_inner);

            if let Some(proc_macro2::TokenTree::Group(steps_g)) = tokens.get(*i) {
                if steps_g.delimiter() == proc_macro2::Delimiter::Brace {
                    *i += 1;
                    let steps_inner: Vec<_> = steps_g.stream().into_iter().collect();
                    let steps = parse_steps(&steps_inner);
                    return Some(StepDef::WithEnv(WithEnvDef { env: env_vars, steps }));
                }
            }
        }
    }
    None
}

fn parse_let_output_step(tokens: &[proc_macro2::TokenTree], i: &mut usize) -> Option<StepDef> {
    // Parse: let_output VAR = step
    if let Some(proc_macro2::TokenTree::Ident(var_id)) = tokens.get(*i) {
        let var_name = var_id.to_string();
        *i += 1;

        // Expect '='
        if let Some(proc_macro2::TokenTree::Punct(p)) = tokens.get(*i) {
            if p.to_string() == "=" {
                *i += 1;
                // Parse the inner step
                if let Some(inner_step) = parse_step(tokens, i) {
                    return Some(StepDef::LetOutput(LetOutputDef {
                        var_name,
                        inner: Box::new(inner_step),
                    }));
                }
            }
        }
    }
    None
}

// =============================================================================
// Code Generation - PipelineSpec + describe_to_stdout
// =============================================================================

fn generate_step_spec(step: &StepDef) -> proc_macro2::TokenStream {
    match step {
        StepDef::Shell(shell) => {
            let script = shell.script.clone();
            let capture_stdout = shell.capture_stdout;
            let return_status = shell.return_status;
            let fail_on_nonzero = shell.fail_on_nonzero;

            let kind = shell.kind.as_ref().map(|k| {
                match k.as_str() {
                    "Sh" | "sh" => quote! { pipeliner_core::spec::ShellKind::Sh },
                    "PowerShell" | "power_shell" => quote! { pipeliner_core::spec::ShellKind::PowerShell },
                    "Cmd" | "cmd" => quote! { pipeliner_core::spec::ShellKind::Cmd },
                    _ => quote! { pipeliner_core::spec::ShellKind::Sh },
                }
            }).unwrap_or(quote! { pipeliner_core::spec::ShellKind::Sh });

            let label = shell.label.as_ref().map(|l| quote! { Some(#l.to_string()) }).unwrap_or(quote! { None });

            let interpolation = shell.interpolation.as_ref().map(|im| {
                match im.as_str() {
                    "raw" => quote! { pipeliner_core::spec::InterpolationMode::Raw },
                    _ => quote! { pipeliner_core::spec::InterpolationMode::Pipeliner },
                }
            }).unwrap_or(quote! { pipeliner_core::spec::InterpolationMode::Pipeliner });

            quote! {
                pipeliner_core::spec::StepSpec::Shell(
                    pipeliner_core::spec::ShellStepSpec {
                        kind: #kind,
                        script: #script.to_string(),
                        label: #label,
                        interpolation: #interpolation,
                        capture_stdout: #capture_stdout,
                        return_status: #return_status,
                        fail_on_nonzero: #fail_on_nonzero,
                    }
                )
            }
        }
        StepDef::Echo(msg) => quote! {
            pipeliner_core::spec::StepSpec::Echo(
                pipeliner_core::spec::EchoStepSpec { message: #msg.to_string() }
            )
        },
        StepDef::Dir(dir) => {
            let path = &dir.path;
            let step_specs = dir.steps.iter().map(generate_step_spec).collect::<Vec<_>>();
            quote! {
                pipeliner_core::spec::StepSpec::Dir(
                    pipeliner_core::spec::DirStepSpec {
                        path: #path.to_string(),
                        steps: vec![#(#step_specs),*],
                    }
                )
            }
        }
        StepDef::WithEnv(with_env) => {
            let env_spec = generate_env_spec(&with_env.env);
            let step_specs = with_env.steps.iter().map(generate_step_spec).collect::<Vec<_>>();
            quote! {
                pipeliner_core::spec::StepSpec::WithEnv(
                    pipeliner_core::spec::WithEnvStepSpec {
                        env: #env_spec,
                        steps: vec![#(#step_specs),*],
                    }
                )
            }
        }
        StepDef::LetOutput(let_output) => {
            let var_name = &let_output.var_name;
            let inner_spec = generate_step_spec(&let_output.inner);
            quote! {
                pipeliner_core::spec::StepSpec::LetOutput(
                    pipeliner_core::spec::LetOutputStepSpec {
                        var_name: #var_name.to_string(),
                        inner: Box::new(#inner_spec),
                    }
                )
            }
        }
    }
}

fn generate_env_spec(env_vars: &[(String, String)]) -> proc_macro2::TokenStream {
    let mut env_spec = quote! { pipeliner_core::spec::EnvSpec::new() };
    for (k, v) in env_vars {
        env_spec = quote! { #env_spec.with_var(#k, #v) };
    }
    env_spec
}

fn generate_options_spec(opts: &OptionsDef) -> proc_macro2::TokenStream {
    let timeout = if let Some(secs) = opts.timeout_secs {
        quote! { Some(std::time::Duration::from_secs(#secs)) }
    } else {
        quote! { None }
    };

    let retry = opts.retry.unwrap_or(0);

    quote! {
        pipeliner_core::spec::OptionsSpec {
            timeout: #timeout,
            retry: #retry,
        }
    }
}

fn generate_stage_execution(content: &StageContent) -> proc_macro2::TokenStream {
    match content {
        StageContent::Steps(steps) => {
            let step_specs = steps.iter().map(generate_step_spec).collect::<Vec<_>>();
            quote! {
                pipeliner_core::spec::StageExecution::Steps {
                    steps: vec![#(#step_specs),*]
                }
            }
        }
        StageContent::Parallel(sub) => {
            let sub_exprs: Vec<_> = sub.iter().map(|s| {
                let nm = &s.name;
                let env = s.env.as_ref().map(|e| generate_env_spec(e)).unwrap_or(quote! { None });
                let opts = s.options.as_ref().map(|o| generate_options_spec(o)).unwrap_or(quote! { None });
                let exec = generate_stage_execution(&s.content);
                quote! {
                    pipeliner_core::spec::StageSpec {
                        id: #nm.to_string(),
                        display_name: #nm.to_string(),
                        env: #env,
                        options: #opts,
                        execution: #exec,
                        post: None,
                    }
                }
            }).collect();
            quote! {
                pipeliner_core::spec::StageExecution::Parallel {
                    stages: vec![#(#sub_exprs),*]
                }
            }
        }
    }
}

pub fn build_pipeline_spec(pipeline: &PipelineDef) -> proc_macro2::TokenStream {
    let env = pipeline.env.as_ref().map(|e| generate_env_spec(e)).unwrap_or(quote! { None });

    let stage_specs: Vec<_> = pipeline.stages.iter().map(|s| {
        let nm = &s.name;
        let stage_env = s.env.as_ref().map(|e| generate_env_spec(e)).unwrap_or(quote! { None });
        let stage_opts = s.options.as_ref().map(|o| generate_options_spec(o)).unwrap_or(quote! { None });
        let exec = generate_stage_execution(&s.content);
        quote! {
            pipeliner_core::spec::StageSpec {
                id: #nm.to_string(),
                display_name: #nm.to_string(),
                env: #stage_env,
                options: #stage_opts,
                execution: #exec,
                post: None,
            }
        }
    }).collect();

    quote! {
        pipeliner_core::spec::PipelineSpec {
            schema_version: "pipeliner.pipeline.v1".to_string(),
            pipeliner_version: "0.1.0".to_string(),
            env: #env,
            stages: vec![#(#stage_specs),*],
            post: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_echo_step() {
        let tokens: Vec<_> = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("echo", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("hello")),
        ];
        let steps = parse_steps(&tokens);
        assert_eq!(steps.len(), 1);
        assert!(matches!(steps[0], StepDef::Echo(ref msg) if msg == "hello"));
    }

    #[test]
    fn test_parse_simple_sh() {
        let tokens: Vec<_> = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("sh", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("echo hi")),
        ];
        let steps = parse_steps(&tokens);
        assert_eq!(steps.len(), 1);
        match &steps[0] {
            StepDef::Shell(shell) => assert_eq!(shell.script, "echo hi"),
            _ => panic!("Expected Shell step"),
        }
    }

    #[test]
    fn test_parse_sh_raw() {
        let tokens: Vec<_> = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("sh", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("raw", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("${VAR}")),
        ];
        let steps = parse_steps(&tokens);
        assert_eq!(steps.len(), 1);
        match &steps[0] {
            StepDef::Shell(shell) => {
                assert_eq!(shell.script, "${VAR}");
                assert_eq!(shell.interpolation.as_deref(), Some("raw"));
            }
            _ => panic!("Expected Shell step"),
        }
    }

    #[test]
    fn test_parse_dir_step() {
        let tokens: Vec<_> = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("dir", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("/tmp")),
            proc_macro2::TokenTree::Group(proc_macro2::Group::new(
                proc_macro2::Delimiter::Brace,
                vec![
                    proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("sh", proc_macro2::Span::call_site())),
                    proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("ls")),
                ].into_iter().collect::<proc_macro2::TokenStream>().into(),
            )),
        ];
        let steps = parse_steps(&tokens);
        assert_eq!(steps.len(), 1);
        match &steps[0] {
            StepDef::Dir(dir) => {
                assert_eq!(dir.path, "/tmp");
                assert_eq!(dir.steps.len(), 1);
            }
            _ => panic!("Expected Dir step"),
        }
    }

    #[test]
    fn test_parse_with_env_step() {
        let env_tokens: proc_macro2::TokenStream = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("FOO", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new('=', proc_macro2::Spacing::Alone)),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("bar")),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(';', proc_macro2::Spacing::Alone)),
        ].into_iter().collect::<proc_macro2::TokenStream>().into();

        let steps_tokens: proc_macro2::TokenStream = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("echo", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("test")),
        ].into_iter().collect::<proc_macro2::TokenStream>().into();

        let tokens: Vec<_> = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("with_env", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Group(proc_macro2::Group::new(proc_macro2::Delimiter::Brace, env_tokens)),
            proc_macro2::TokenTree::Group(proc_macro2::Group::new(proc_macro2::Delimiter::Brace, steps_tokens)),
        ];

        let steps = parse_steps(&tokens);
        assert_eq!(steps.len(), 1);
        match &steps[0] {
            StepDef::WithEnv(with_env) => {
                assert_eq!(with_env.env.len(), 1);
                assert_eq!(with_env.steps.len(), 1);
            }
            _ => panic!("Expected WithEnv step"),
        }
    }

    #[test]
    fn test_parse_let_output_step() {
        let tokens: Vec<_> = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("let_output", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("RESULT", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new('=', proc_macro2::Spacing::Alone)),
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("sh", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("echo hello")),
        ];

        let steps = parse_steps(&tokens);
        assert_eq!(steps.len(), 1);
        match &steps[0] {
            StepDef::LetOutput(let_output) => {
                assert_eq!(let_output.var_name, "RESULT");
            }
            _ => panic!("Expected LetOutput step"),
        }
    }

    #[test]
    fn test_parse_env_block() {
        let tokens: Vec<_> = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("FOO", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new('=', proc_macro2::Spacing::Alone)),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("bar")),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(';', proc_macro2::Spacing::Alone)),
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("BAZ", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new('=', proc_macro2::Spacing::Alone)),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("qux")),
        ];

        let env = parse_env_content(&tokens);
        assert_eq!(env.len(), 2);
        assert_eq!(env[0], ("FOO".to_string(), "bar".to_string()));
        assert_eq!(env[1], ("BAZ".to_string(), "qux".to_string()));
    }

    #[test]
    fn test_parse_options_block() {
        // Parse tokens for: timeout(minutes(5)); retry(3);
        let timeout_tokens: proc_macro2::TokenStream = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("minutes", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Group(proc_macro2::Group::new(
                proc_macro2::Delimiter::Parenthesis,
                vec![
                    proc_macro2::TokenTree::Literal(proc_macro2::Literal::u64_unsuffixed(5)),
                ].into_iter().collect::<proc_macro2::TokenStream>().into(),
            )),
        ].into_iter().collect::<proc_macro2::TokenStream>().into();

        let retry_tokens: proc_macro2::TokenStream = vec![
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::u32_unsuffixed(3)),
        ].into_iter().collect::<proc_macro2::TokenStream>().into();

        let tokens: Vec<_> = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("timeout", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Group(proc_macro2::Group::new(
                proc_macro2::Delimiter::Parenthesis,
                timeout_tokens,
            )),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(';', proc_macro2::Spacing::Alone)),
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("retry", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Group(proc_macro2::Group::new(
                proc_macro2::Delimiter::Parenthesis,
                retry_tokens,
            )),
        ];

        let opts = parse_options_content(&tokens);
        assert_eq!(opts.timeout_secs, Some(300)); // 5 minutes = 300 seconds
        assert_eq!(opts.retry, Some(3));
    }

    #[test]
    fn test_parse_sh_block_full() {
        let tokens: Vec<_> = vec![
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("script", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("echo hello")),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(';', proc_macro2::Spacing::Alone)),
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("label", proc_macro2::Span::call_site())),
            proc_macro2::TokenTree::Literal(proc_macro2::Literal::string("my step")),
            proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(';', proc_macro2::Spacing::Alone)),
            proc_macro2::TokenTree::Ident(proc_macro2::Ident::new("capture_stdout", proc_macro2::Span::call_site())),
        ];

        let shell = parse_sh_block(&tokens);
        assert_eq!(shell.script, "echo hello");
        assert_eq!(shell.label, Some("my step".to_string()));
        assert!(shell.capture_stdout);
    }
}

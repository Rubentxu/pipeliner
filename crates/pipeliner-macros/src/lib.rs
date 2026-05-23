//! Pipeliner DSL - Jenkins-style declarative pipelines
//!
//! NEW syntax (restructuring-mvp-0):
//! ```ignore
//! pipeline! {
//!     env { FOO = "bar"; }
//!     stages {
//!         stage "Build" {
//!             env { BUILD_ENV = "prod"; }
//!             options { timeout(minutes(5)); retry(3); }
//!             steps {
//!                 sh "cargo build";
//!                 sh raw "${UNCHANGED_VAR}";
//!                 sh {
//!                     script = "echo hello";
//!                     label = "greeting";
//!                     capture_stdout;
//!                 };
//!                 dir "/tmp" {
//!                     sh "ls";
//!                 }
//!                 with_env { FOO = "bar"; } {
//!                     sh "echo $FOO";
//!                 }
//!                 let_output RESULT = sh "echo output";
//!                 echo "done";
//!             }
//!         }
//!         stage "Test" {
//!             parallel {
//!                 stage "Unit" { steps { sh "cargo test"; } }
//!                 stage "Integration" { steps { sh "cargo integration-test"; } }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! The macro generates a main() that calls pipeliner_protocol::describe_to_stdout(&spec)

mod pipeline_parsing;

// Pipeline macro must be at crate root
use proc_macro::TokenStream;
use quote::quote;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro]
pub fn pipeline(input: TokenStream) -> TokenStream {
    let input: TokenStream2 = input.into();
    let tokens: Vec<_> = input.into_iter().collect();
    let pipeline_parse = pipeline_parsing::parse_pipeline(tokens);

    if pipeline_parse.stages.is_empty() {
        return syn::Error::new_spanned(
            &proc_macro2::TokenTree::Group(proc_macro2::Group::new(
                proc_macro2::Delimiter::Brace,
                proc_macro2::TokenStream::new()
            )),
            "No stages found in pipeline!"
        ).to_compile_error().into();
    }

    let pipeline_spec = pipeline_parsing::build_pipeline_spec(&pipeline_parse);

    // Generate main that outputs PipelineSpec JSON to stdout
    // Using serde_json directly so script only needs serde_json dep
    quote! {
        fn main() {
            let spec = #pipeline_spec;
            serde_json::to_writer(std::io::stdout(), &spec).unwrap();
        }
    }.into()
}

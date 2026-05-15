//! Stage macro con soporte para bloques
//!
//! `stage!("Build") { steps { ... }`

use proc_macro::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::Parser,
    Expr, ExprBlock, Item, LitStr, Token,
};
use std::str::FromStr;

#[proc_macro]
pub fn stage_macro(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as StageMacroInput);
    quote!(Stage::new(#name).with_steps(#steps)
}

struct StageMacroInput {
    name: LitStr,
    steps: Vec<Step>,
}

impl Parse for StageMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let content;
        let brace = syn::braced!(content in input);
        let steps = content.parse_terminated(Step::parse)?;
        Ok(Self { name, steps: steps.into_iter().collect() })
    }
}

#[test]
fn test_stage_macro_compiles() {
    let ts: TokenStream = parse_quote! {
        stage_macro!("Build", { sh!("make build") })
    };
    let expanded = proc_macro2::TokenStream::from(ts);
    assert!(expanded.to_string().contains("Stage::new"));
}

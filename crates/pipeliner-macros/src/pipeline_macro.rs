//! Pipeline macro con soporte para bloques estilo Jenkinsfile
//!
//! ```rust
//! pipeline! {
//!     agent(any)
//!     stages {
//!         stage!("Build", vec![sh!("make build")])
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_quote, Expr, Ident, Token};
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro]
pub fn pipeline(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    
    // Parser simple: solo acepta pipeline! { ... }
    // Formato: pipeline! { agent(any) stages { ... } post { ... } }
    let tokens: TokenStream2 = input.parse().unwrap_or_else(|_| {
        // Fallback: Pipeline::new()
        quote! { Pipeline::new() }
    });
    
    quote! {
        Pipeline::new()
            #tokens
    }.into()
}

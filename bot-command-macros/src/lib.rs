use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

/// Defines a Poise command and registers it as a top-level command.
#[proc_macro_attribute]
pub fn bot_command(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let function = parse_macro_input!(item as ItemFn);
    let name = &function.sig.ident;
    let cfg = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"));

    quote! {
        #[::poise::command(#args)]
        #function

        #(#cfg)*
        ::inventory::submit! {
            crate::commands::CommandFactory(#name)
        }
    }
    .into()
}

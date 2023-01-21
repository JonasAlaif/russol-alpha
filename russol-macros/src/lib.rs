#![no_std]
#![feature(box_patterns, box_syntax)]
#![feature(let_chains)]
#![feature(proc_macro_span, proc_macro_diagnostic, proc_macro_span_shrink)]

extern crate alloc;

use alloc::string::ToString;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::parse_quote;

mod rewriter;

#[proc_macro_attribute]
pub fn requires(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    parse_fn_specs(tokens.into(), attr.into(), SpecKind::Requires)
        .unwrap_or_else(|x| x)
        .into()
}

#[proc_macro_attribute]
pub fn ensures(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    parse_fn_specs(tokens.into(), attr.into(), SpecKind::Ensures)
        .unwrap_or_else(|x| x)
        .into()
}

#[proc_macro_attribute]
pub fn trusted_ensures(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    parse_fn_specs(tokens.into(), attr.into(), SpecKind::TrustedEnsures)
        .unwrap_or_else(|x| x)
        .into()
}

#[proc_macro_attribute]
pub fn pure(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match syn::parse2::<syn::ItemFn>(tokens.into()) {
        Ok(mut item_fn) => {
            item_fn.attrs.push(parse_quote! { #[rustfmt::ruslik_pure] });
            item_fn.into_token_stream()
        }
        Err(e) => e.to_compile_error(),
    }
    .into()
}

#[proc_macro_attribute]
pub fn helper(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match syn::parse2::<syn::ItemFn>(tokens.into()) {
        Ok(mut item_fn) => {
            item_fn
                .attrs
                .push(parse_quote! { #[rustfmt::ruslik_helper] });
            item_fn.into_token_stream()
        }
        Err(e) => e.to_compile_error(),
    }
    .into()
}

#[proc_macro_attribute]
pub fn synth(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match syn::parse2::<syn::ItemFn>(tokens.into()) {
        Ok(mut item_fn) => {
            item_fn
                .attrs
                .push(parse_quote! { #[rustfmt::ruslik_synth] });
            item_fn.into_token_stream()
        }
        Err(e) => e.to_compile_error(),
    }
    .into()
}

#[proc_macro_attribute]
pub fn params(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match syn::parse2::<syn::ItemFn>(tokens.into()) {
        Ok(mut item_fn) => {
            let attr: TokenStream2 = attr.into();
            item_fn
                .attrs
                .push(parse_quote! { #[rustfmt::ruslik_params = #attr] });
            item_fn.into_token_stream()
        }
        Err(e) => e.to_compile_error(),
    }
    .into()
}

#[proc_macro_attribute]
pub fn extern_spec(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match syn::parse2::<syn::ItemFn>(tokens.into()) {
        Ok(mut item_fn) => {
            item_fn
                .attrs
                .push(parse_quote! { #[rustfmt::ruslik_extern_spec] });
            item_fn.into_token_stream()
        }
        Err(e) => e.to_compile_error(),
    }
    .into()
}

#[derive(Clone, Copy)]
pub(crate) enum SpecKind {
    Requires,
    Ensures,
    TrustedEnsures,
}
fn parse_fn_specs(
    fun: TokenStream2,
    attr: TokenStream2,
    attr_kind: SpecKind,
) -> Result<TokenStream2, TokenStream2> {
    let mut item_fn: syn::ItemFn = syn::parse2(fun).map_err(|e| e.to_compile_error())?;
    let orig_len = item_fn.block.stmts.len();
    item_fn.block.stmts.insert(
        0,
        rewriter::parse_attr(attr, attr_kind, &item_fn.sig.output)?,
    );

    item_fn.attrs = item_fn
        .attrs
        .into_iter()
        .filter_map(|attr| {
            let sk = match attr.path.segments[0].ident.to_string().as_str() {
                "requires" => SpecKind::Requires,
                "ensures" => SpecKind::Ensures,
                "ruslik_spec_count" => panic!("The attribute `ruslik_spec_count` is reserved!"),
                _ => return Some(Ok(attr)),
            };
            match rewriter::parse_attr(attr.tokens, sk, &item_fn.sig.output) {
                Ok(expr) => {
                    item_fn.block.stmts.insert(0, expr);
                    None
                }
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<Result<_, _>>()?;

    let spec_count = (item_fn.block.stmts.len() - orig_len).to_string();
    // Either we register `ruslik` as a tool (but this runs into issues when compiling without ruslic)
    // Or we hijack a built-in tool for attributes (`rustfmt` or `clippy`)
    item_fn
        .attrs
        .push(parse_quote! { #[rustfmt::ruslik_spec_count = #spec_count] });
    Ok(item_fn.into_token_stream())
}

#[proc_macro]
pub fn ruslik(input: TokenStream) -> TokenStream {
    let input: TokenStream2 = input.into();
    quote::quote! { panic!(#input) }.into()
}

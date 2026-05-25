#![deny(missing_docs)]

//! Proc-macro helpers for effie. Provides compile-time UTF-16 string construction.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

/// Compile-time `&WStr` literal.
///
/// Converts a Rust string literal into a statically allocated `&'static WStr` at compile time
/// by encoding it as UTF-16 and appending a null terminator.
///
/// # Example
/// ```ignore
/// use effie::w;
/// let hello: &WStr = w!("Hello, UEFI!");
/// ```
#[proc_macro]
pub fn w(input: TokenStream) -> TokenStream {
    let lit: LitStr = parse_macro_input!(input);

    let encoded = lit.value().encode_utf16().collect::<Vec<u16>>();

    quote! {
        unsafe {::effie::WStr::from_slice_unchecked(&[#( #encoded, )* 0u16])}
    }
    .into()
}

/// Internal variant of `w!` that uses `crate::` paths. Not part of the public API.
#[proc_macro]
#[doc(hidden)]
pub fn w_internal(input: TokenStream) -> TokenStream {
    let lit: LitStr = parse_macro_input!(input);

    let encoded = lit.value().encode_utf16().collect::<Vec<u16>>();

    quote! {
       unsafe { crate::WStr::from_slice_unchecked(&[#( #encoded, )* 0u16])}
    }
    .into()
}

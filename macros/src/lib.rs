#![feature(proc_macro_value)]
#![deny(missing_docs)]

//! Proc-macro helpers for effie. Provides compile-time UTF-16 string construction.

use std::fmt::Write as _;

use proc_macro::{Delimiter, TokenStream, TokenTree};

fn compile_error(message: &str) -> TokenStream {
    let mut output = String::from("compile_error!(");
    write!(&mut output, "{message:?}").expect("writing to a String cannot fail");
    output.push(')');
    output
        .parse()
        .expect("effie-macros generated an invalid compile_error invocation")
}

fn string_literal(input: TokenStream) -> Result<String, TokenStream> {
    let mut tokens = input.into_iter();
    let token = tokens
        .next()
        .ok_or_else(|| compile_error("expected exactly one string literal"))?;

    if tokens.next().is_some() {
        return Err(compile_error("expected exactly one string literal"));
    }

    match token {
        TokenTree::Group(group) if group.delimiter() == Delimiter::None => {
            string_literal(group.stream())
        }
        TokenTree::Literal(literal) => literal
            .str_value()
            .map_err(|_| compile_error("expected a string literal")),
        _ => Err(compile_error("expected a string literal")),
    }
}

fn encoded_literal(input: TokenStream) -> Result<Vec<u16>, TokenStream> {
    let value = string_literal(input)?;

    if value.contains('\0') {
        return Err(compile_error(
            "wide string literals cannot contain an interior NUL",
        ));
    }

    let mut encoded = value.encode_utf16().collect::<Vec<_>>();
    encoded.push(0);
    Ok(encoded)
}

fn encoded_slice(encoded: &[u16]) -> String {
    let mut output = String::from("&[");
    for unit in encoded {
        write!(&mut output, "{unit}u16,").expect("writing to a String cannot fail");
    }
    output.push(']');
    output
}

/// Encodes a string literal as a reference to a null-terminated UTF-16 array.
#[proc_macro]
#[doc(hidden)]
pub fn wstr_units(input: TokenStream) -> TokenStream {
    match encoded_literal(input) {
        Ok(encoded) => encoded_slice(&encoded)
            .parse()
            .expect("effie-macros generated invalid Rust tokens"),
        Err(error) => error,
    }
}

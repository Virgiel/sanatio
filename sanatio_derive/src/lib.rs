use std::{convert::TryInto, fmt::format, str::FromStr};

use proc_macro::{token_stream, TokenStream};
use proc_macro2::Ident;
use proc_macro_error::{abort, emit_error, emit_warning, proc_macro_error};
use quote::{format_ident, quote, ToTokens, __private::ext::RepToTokensExt};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token::Comma,
    DeriveInput, Expr, Lit, Meta, MetaList, Token,
};

#[proc_macro_derive(Validate, attributes(validate))]
#[proc_macro_error]
pub fn derive(input: proc_macro::TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let attrs: Vec<_> = input
        .attrs
        .iter()
        .filter(|it| it.path.to_token_stream().to_string() == "validate")
        .collect();
    let mut validation = None;
    if attrs.len() > 1 {
        abort!(
            attrs.last().unwrap(),
            "cannot have more than one validation step"
        )
    } else if let Some(attr) = attrs.first() {
        validation = Some(&attr.tokens);
    }

    let fields = if let syn::Data::Struct(syn::DataStruct { fields, .. }) = input.data {
        fields
            .into_iter()
            .map(|it| (it.attrs, it.ident.unwrap(), it.ty))
            .collect::<Vec<_>>()
    } else {
        abort!(name, "#[derive(Validate)] only work on named struct")
    };
    let fields: Vec<_> = fields
        .into_iter()
        .map(|(attrs, ident, ty)| {
            let attrs: Vec<_> = attrs
                .iter()
                .filter(|it| it.path.to_token_stream().to_string() == "validate")
                .collect();
            if attrs.len() > 1 {
                abort!(
                    attrs.last().unwrap(),
                    "cannot have more than one validation step"
                )
            } else if let Some(attr) = attrs.first() {
                let ugly = &attr.tokens;
                let ugly = ugly.into_token_stream().to_string();
                let ugly = ugly.trim_matches(|c| c == '(' || c == ')').split(',');

                let mut serde = proc_macro2::TokenStream::default();
                let mut fun = None;
                let mut in_ty = None;
                for word in ugly {
                    let stream = proc_macro2::TokenStream::from_str(word).unwrap();
                    if word == "flatten" {
                        serde = proc_macro2::TokenStream::from_str("#[serde(flatten)]").unwrap();
                    } else if fun.is_none() {
                        fun = Some(stream)
                    } else if in_ty.is_none() {
                        in_ty = Some(stream)
                    } else {
                        abort!(attr, "to much args")
                    }
                }

                let ty = in_ty.unwrap_or_else(|| ty.to_token_stream());
                (ident, ty, fun.expect("Missing validation function"), serde)
            } else {
                abort!(ident, "is missing a validation")
            }
        })
        .collect();
    let names: Vec<_> = fields.iter().map(|(it, _, _, _)| it).collect();
    let types: Vec<_> = fields.iter().map(|(_, it, _, _)| it).collect();
    let action: Vec<_> = fields.iter().map(|(_, _, it, _)| it).collect();
    let serde: Vec<_> = fields.iter().map(|(_, _, _, it)| it).collect();

    let validation = validation.map(|action| {
        quote!(
            let tmp = #action(tmp)?;
        )
    });

    let expanded = quote!(
        impl<'de> serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                #[derive(serde::Deserialize)]
                pub struct Input {
                    #(#serde #names: #types,)*
                }

                impl TryFrom<Input> for #name {
                    type Error = std::borrow::Cow<'static, str>; // Use String as error type just for simplicity

                    fn try_from(v: Input) -> Result<Self, Self::Error> {
                        let tmp = Self {
                            #(#names: #action(v.#names)?,)*
                        };
                        #validation;
                        return Ok(tmp);
                    }
                }


                Result::and_then(
                    <Input as serde::Deserialize>::deserialize(deserializer),
                    |v| TryFrom::try_from(v).map_err(serde::de::Error::custom),
                )
            }
        }


    );
    proc_macro::TokenStream::from(expanded)
}

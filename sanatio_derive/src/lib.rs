use std::str::FromStr;

use proc_macro2::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Validate, attributes(validate, serde))]
#[proc_macro_error]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let fields = if let syn::Data::Struct(syn::DataStruct { fields, .. }) = input.data {
        fields
            .into_iter()
            .map(|it| (it.attrs, it.ident.unwrap(), it.ty))
            .collect::<Vec<_>>()
    } else {
        abort!(name, "#[derive(Validate)] only work on named struct")
    };
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
    let fields: Vec<_> = fields
        .into_iter()
        .map(|(attrs, ident, ty)| {
            let validate: Vec<_> = attrs
                .iter()
                .filter(|it| it.path.to_token_stream().to_string() == "validate")
                .collect();
            let serde: Vec<_> = attrs
                .iter()
                .filter(|it| it.path.to_token_stream().to_string() == "serde")
                .cloned()
                .collect();

            match *validate.as_slice() {
                [attr] => {
                    let args = &attr.tokens;
                    let args = args.into_token_stream().to_string();
                    let args = args.trim_matches(|c| c == '(' || c == ')').split(',');
                    let mut fun = None;
                    let mut in_ty = None;
                    let mut opt = false;
                    for word in args {
                        if word == "opt" {
                            opt = true;
                            continue;
                        }

                        if fun.is_none() {
                            let str = if opt {
                                format!("::sanatio::opt({word})")
                            } else {
                                word.into()
                            };
                            fun = Some(TokenStream::from_str(&str).unwrap())
                        } else if in_ty.is_none() {
                            let str = if opt {
                                format!("Option<{word}>")
                            } else {
                                word.into()
                            };
                            in_ty = Some(TokenStream::from_str(&str).unwrap())
                        } else {
                            abort!(attr, "to much args")
                        }
                    }

                    let ty = in_ty.unwrap_or_else(|| ty.to_token_stream());
                    (ident, ty, fun.expect("Missing validation function"), serde)
                }
                [.., last] => abort!(last, "cannot have more than one validation step"),
                [] => abort!(ident, "is missing a validation"),
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
                    #(#(#serde)* #names: #types,)*
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

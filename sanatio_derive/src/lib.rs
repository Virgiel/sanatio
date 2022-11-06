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
            "cannot have more than one final validation type"
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
                    let args = attr.tokens.to_string();
                    let mut args = args
                        .trim()
                        .strip_prefix('(')
                        .unwrap()
                        .strip_suffix(')')
                        .unwrap()
                        .split(',');
                    let Some(fun) = args.next().map(|s| TokenStream::from_str(s).unwrap()) else {
                        abort!(attr.tokens, "missing validation function")
                    };
                    let ty = args
                        .next()
                        .map(|s| TokenStream::from_str(s).unwrap())
                        .unwrap_or_else(|| ty.to_token_stream());
                    if args.next().is_some() {
                        abort!(attr.tokens, "to many validation args")
                    }
                    (ident, ty, fun, serde)
                }
                [.., last] => abort!(last, "cannot have more than one validation step"),
                [] => abort!(ident, "missing a validation"),
            }
        })
        .collect();
    let names = fields.iter().map(|(it, _, _, _)| it);
    let types = fields.iter().map(|(_, it, _, _)| it);
    let action = fields.iter().map(|(_, _, it, _)| it);
    let serde = fields.iter().map(|(_, _, _, it)| it);
    let names1 = names.clone();
    let names2 = names.clone();

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
                            #(#names1: (#action)(v.#names2)?,)*
                        };
                        #validation
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

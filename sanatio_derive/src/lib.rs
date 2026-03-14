use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Expr, Fields, Result, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Represents the arguments inside `#[validate(action, OptionalType)]`
struct ValidateArgs {
    action: Expr,
    ty: Option<Type>,
}

impl Parse for ValidateArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse the first argument as an expression (e.g., a function path or closure)
        let action: Expr = input.parse()?;

        // Check if there is a comma, and if so, parse the optional Type replacement
        let ty = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        // Ensure there are no leftover, unparsed arguments
        if !input.is_empty() {
            return Err(input.error("too many validation args"));
        }

        Ok(Self { action, ty })
    }
}

#[proc_macro_derive(Validate, attributes(validate, serde))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Delegate to an inner function to utilize the `?` operator for clean error handling
    expand(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand(input: DeriveInput) -> Result<TokenStream2> {
    let name = input.ident;

    // 1. Ensure we are operating on a named struct
    let Data::Struct(data_struct) = input.data else {
        return Err(Error::new_spanned(
            name,
            "#[derive(Validate)] only works on structs",
        ));
    };
    let Fields::Named(fields_named) = data_struct.fields else {
        return Err(Error::new_spanned(
            name,
            "#[derive(Validate)] only works on named structs",
        ));
    };

    // 2. Extract struct-level #[validate(...)] attribute
    let mut struct_validation: Option<Expr> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("validate") {
            if struct_validation.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    "cannot have more than one final validation type",
                ));
            }
            struct_validation = Some(attr.parse_args()?);
        }
    }

    let validation_step = struct_validation.map(|action| {
        quote! { let tmp = (#action)(tmp)?; }
    });

    // 3. Extract and parse field-level attributes
    let mut field_names = Vec::new();
    let mut field_types = Vec::new();
    let mut field_actions = Vec::new();
    let mut field_serde_attrs = Vec::new();

    for field in fields_named.named {
        let ident = field.ident.as_ref().unwrap();
        let mut validate_args: Option<ValidateArgs> = None;
        let mut serde_attrs = Vec::new();

        for attr in &field.attrs {
            if attr.path().is_ident("serde") {
                serde_attrs.push(attr.clone());
            } else if attr.path().is_ident("validate") {
                if validate_args.is_some() {
                    return Err(Error::new_spanned(
                        attr,
                        "cannot have more than one validation step",
                    ));
                }
                // Safely parse tokens using our custom Parse implementation
                validate_args = Some(attr.parse_args::<ValidateArgs>()?);
            }
        }

        let Some(args) = validate_args else {
            return Err(Error::new_spanned(ident, "missing a validation attribute"));
        };

        field_names.push(ident.clone());
        field_types.push(args.ty.unwrap_or(field.ty)); // Fallback to original field type
        field_actions.push(args.action);
        field_serde_attrs.push(serde_attrs);
    }

    // 4. Generate the final output
    let expanded = quote! {
        impl<'de> serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                #[derive(serde::Deserialize)]
                pub struct Input {
                    #(
                        #(#field_serde_attrs)* #field_names: #field_types,
                    )*
                }

                impl TryFrom<Input> for #name {
                    type Error = std::borrow::Cow<'static, str>;

                    fn try_from(v: Input) -> std::result::Result<Self, Self::Error> {
                        let tmp = Self {
                            #( #field_names: (#field_actions)(v.#field_names)?, )*
                        };
                        #validation_step
                        Ok(tmp)
                    }
                }

                std::result::Result::and_then(
                    <Input as serde::Deserialize>::deserialize(deserializer),
                    |v| TryFrom::try_from(v).map_err(serde::de::Error::custom),
                )
            }
        }
    };

    Ok(expanded)
}

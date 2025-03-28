// File: `cache_diff_derive/src/parse_container.rs`

use crate::MACRO_NAME;
use crate::NAMESPACE;
use crate::parse_field::ParseField;
use crate::shared::{ErrorBank, WithSpan};

// Code
/// Container (i.e. struct Metadata { ... }) and its parsed attributes
/// i.e. `#[cache_diff( ... )]`
#[derive(Debug)]
pub(crate) struct ParseContainer {
    /// The proc-macro identifier for a container i.e. `struct Metadata { }` would be a programmatic
    /// reference to `Metadata` that can be used along with `quote!` to produce code.
    pub(crate) ident: syn::Ident,
    /// Info about generics, lifetimes and where clauses i.e. `struct Metadata<T> { name: T }`
    pub(crate) generics: syn::Generics,
    /// An optional path to a custom diff function
    /// Set via attribute on the container i.e. `#[cache_diff(custom = <function>)]`
    pub(crate) custom: Option<syn::Path>,
    /// Fields (i.e. `name: String`) and their associated attributes i.e. `#[cache_diff(...)]`
    pub(crate) fields: Vec<ParseField>,
}

impl ParseContainer {
    pub(crate) fn from_derive_input(input: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let ident = input.ident.clone();
        let generics = input.generics.clone();
        let mut fields = Vec::new();
        let mut errors = ErrorBank::new();
        let mut custom = None;

        // Continue parsing fields even if attribute has an error
        match crate::shared::parse_attrs::<WithSpan<ParseAttribute>>(&input.attrs)
            .and_then(crate::shared::unique)
        {
            Ok(mut unique) => {
                for (_, WithSpan(value, _)) in unique.drain() {
                    match value {
                        ParseAttribute::custom(path) => custom = Some(path),
                    }
                }
            }
            Err(error) => errors.push_back(error),
        };

        let syn_fields = match input.data {
            syn::Data::Struct(syn::DataStruct {
                fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
                ..
            }) => Ok(named),
            _ => Err(syn::Error::new(
                ident.span(),
                format!("{MACRO_NAME} can only be used on named structs"),
            )),
        }?;

        for syn_field in syn_fields.iter() {
            match ParseField::from_field(syn_field) {
                Ok(ParseField {
                    ignore: Some(value),
                    ..
                }) => {
                    if value == "custom" && custom.is_none() {
                        errors.push_back(syn::Error::new(
                            ident.span(),
                            format!(
                                "field `{field}` on {container} marked ignored as custom, but missing `#[{NAMESPACE}({custom_attr})]` found on `{container}`",
                                field = syn_field.clone().ident.expect("named structs only"),
                                container = &ident,
                                custom_attr = KnownAttribute::custom,
                            )
                        ))
                    } else {
                        // Field is ignored
                    }
                }
                Ok(active_field) => fields.push(active_field),
                Err(error) => {
                    errors.push_back(error);
                }
            }
        }

        if let Some(error) = crate::shared::combine(errors) {
            Err(error)
        } else if fields.is_empty() {
            Err(syn::Error::new(
                ident.span(),
                format!(
                    "No fields to compare for {MACRO_NAME}, ensure struct has at least one named field that isn't `{NAMESPACE}({ignore_attr})`",
                    ignore_attr = crate::parse_field::KnownAttribute::ignore
                ),
            ))
        } else {
            Ok(ParseContainer {
                ident,
                generics,
                custom,
                fields,
            })
        }
    }
}

/// A single attribute
#[derive(strum::EnumDiscriminants, Debug, PartialEq)]
#[strum_discriminants(
    name(KnownAttribute),
    derive(strum::EnumIter, strum::Display, strum::EnumString, Hash)
)]
enum ParseAttribute {
    #[allow(non_camel_case_types)]
    custom(syn::Path), // #[cache_diff(custom=<function>)]
}

impl syn::parse::Parse for KnownAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let identity: syn::Ident = input.parse()?;
        crate::shared::known_attribute(&identity)
    }
}

impl syn::parse::Parse for ParseAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key: KnownAttribute = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        match key {
            KnownAttribute::custom => Ok(ParseAttribute::custom(input.parse()?)),
        }
    }
}

#[cfg(test)]
mod tests {
    // Test use
    use super::*;
    // Test code
    #[test]
    fn test_parses() {
        let container = ParseContainer::from_derive_input(&syn::parse_quote! {
            struct Metadata {
                version: String
            }
        })
        .unwrap();
        assert_eq!(1, container.fields.len());

        let container = ParseContainer::from_derive_input(&syn::parse_quote! {
            struct Metadata {
                version: String,
                checksum: String
            }
        })
        .unwrap();
        assert_eq!(2, container.fields.len());
    }

    #[test]
    fn test_captures_many_field_errors() {
        let result = ParseContainer::from_derive_input(&syn::parse_quote! {
            struct Metadata {
                #[cache_diff(unknown)]
                #[cache_diff(unknown)]
                version: String,

                #[cache_diff(unknown)]
                #[cache_diff(unknown)]
                name: String
            }
        });

        assert!(
            result.is_err(),
            "Expected {result:?} to be err but it is not"
        );
        let error = result.err().unwrap();
        assert_eq!(4, error.into_iter().count());
    }

    #[test]
    fn test_known_attributes() {
        let attribute: KnownAttribute = syn::parse_str("custom").unwrap();
        assert_eq!(KnownAttribute::custom, attribute);
    }

    #[test]
    fn test_parse_attribute() {
        let attribute: ParseAttribute = syn::parse_str("custom = my_function").unwrap();
        assert!(matches!(attribute, ParseAttribute::custom(_)));

        let result: Result<ParseAttribute, syn::Error> = syn::parse_str("unknown");
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            r"Unknown cache_diff attribute: `unknown`. Must be one of `custom`",
            format!("{}", result.err().unwrap()),
        );
    }

    #[test]
    fn test_custom_parse_attribute() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[cache_diff(custom = my_function)]
            struct Metadata {
                name: String
            }
        };

        assert!(matches!(
            crate::shared::parse_attrs::<ParseAttribute>(&input.attrs)
                .unwrap()
                .first(),
            Some(ParseAttribute::custom(_))
        ));
    }

    #[test]
    fn test_no_fields() {
        let result = ParseContainer::from_derive_input(&syn::parse_quote! {
            struct Metadata { }
        });
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"No fields to compare for CacheDiff, ensure struct has at least one named field that isn't `cache_diff(ignore)`"#
        );
    }

    #[test]
    fn test_all_ignored() {
        let result = ParseContainer::from_derive_input(&syn::parse_quote! {
            struct Metadata {
                #[cache_diff(ignore)]
                version: String
            }
        });
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"No fields to compare for CacheDiff, ensure struct has at least one named field that isn't `cache_diff(ignore)`"#
        );
    }
}

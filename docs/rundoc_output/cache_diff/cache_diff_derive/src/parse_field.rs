// File: `cache_diff_derive/src/parse_field.rs`

use crate::MACRO_NAME;
use crate::shared::{ErrorBank, WithSpan};
use syn::spanned::Spanned;

// Code
/// Field (i.e. `name: String`) of a container (struct) and its parsed attributes
/// i.e. `#[cache_diff(rename = "Ruby version")]`
#[derive(Debug)]
pub(crate) struct ParseField {
    /// The proc-macro identifier for a field i.e. `name: String` would be a programmatic
    /// reference to `name` that can be used along with `quote!` to produce code.
    pub(crate) ident: syn::Ident,
    /// What the user will see when this field differs and invalidates the cache
    /// i.e. `age: usize` will be `"age"`.
    pub(crate) name: String,
    /// Whether or not the field is included in the derived diff comparison
    pub(crate) ignore: Option<String>,
    /// The function to use when rendering values on the field
    /// i.e. `age: 42` will be `"42"`
    pub(crate) display: syn::Path,
}

impl ParseField {
    pub(crate) fn from_field(field: &syn::Field) -> Result<Self, syn::Error> {
        let mut errors = ErrorBank::new();
        let mut rename = None;
        let mut ignore = None;
        let mut display = None;
        // If un-named field, we cannot continue. Return with `?`
        let ident = field.ident.clone().ok_or_else(|| {
            syn::Error::new(
                field.span(),
                format!("{MACRO_NAME} can only be used on structs with named fields"),
            )
        })?;

        // If Syntax error we cannot continue. Return with `?`
        let attributes = crate::shared::parse_attrs::<WithSpan<ParseAttribute>>(&field.attrs)?;
        if let Err(error) = crate::shared::check_exclusive(KnownAttribute::ignore, &attributes) {
            errors.push_back(error);
        }

        match crate::shared::unique(attributes) {
            Ok(mut unique) => {
                for (_, WithSpan(attribute, span)) in unique.drain() {
                    match attribute {
                        ParseAttribute::rename(inner) => rename = Some(inner),
                        ParseAttribute::ignore(inner) => ignore = Some((inner, span)),
                        ParseAttribute::display(inner) => display = Some(inner),
                    }
                }
            }
            Err(error) => errors.push_back(error),
        }

        if let Some(error) = crate::shared::combine(errors) {
            Err(error)
        } else {
            let name = rename.unwrap_or_else(|| ident.to_string().replace("_", " "));
            let display = display.unwrap_or_else(|| {
                if is_pathbuf(&field.ty) {
                    syn::parse_str("std::path::Path::display")
                        .expect("PathBuf::display parses as a syn::Path")
                } else {
                    syn::parse_str("std::convert::identity")
                        .expect("std::convert::identity parses as a syn::Path")
                }
            });
            let ignore = ignore.map(|(ignore, _)| ignore);

            Ok(ParseField {
                ident,
                name,
                ignore,
                display,
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
pub(crate) enum ParseAttribute {
    #[allow(non_camel_case_types)]
    rename(String), // #[cache_diff(rename="...")]
    #[allow(non_camel_case_types)]
    display(syn::Path), // #[cache_diff(display=<function>)]
    #[allow(non_camel_case_types)]
    ignore(String), // #[cache_diff(ignore)]
}

impl syn::parse::Parse for KnownAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let identity = input.parse::<syn::Ident>()?;
        crate::shared::known_attribute(&identity)
    }
}

impl syn::parse::Parse for ParseAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key: KnownAttribute = input.parse()?;

        match key {
            KnownAttribute::rename => {
                input.parse::<syn::Token![=]>()?;
                Ok(ParseAttribute::rename(
                    input.parse::<syn::LitStr>()?.value(),
                ))
            }
            KnownAttribute::display => {
                input.parse::<syn::Token![=]>()?;
                Ok(ParseAttribute::display(input.parse()?))
            }
            KnownAttribute::ignore => {
                if input.peek(syn::Token![=]) {
                    input.parse::<syn::Token![=]>()?;
                    Ok(ParseAttribute::ignore(
                        input.parse::<syn::LitStr>()?.value(),
                    ))
                } else {
                    Ok(ParseAttribute::ignore("default".to_string()))
                }
            }
        }
    }
}

fn is_pathbuf(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "PathBuf" && segment.arguments == syn::PathArguments::None;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    // Test use
    use super::*;
    // Test code
    #[test]
    fn test_parse_field_plain() {
        let field: syn::Field = syn::parse_quote! {
            ruby_version: String
        };

        let parsed = ParseField::from_field(&field).unwrap();
        assert_eq!("ruby version".to_string(), parsed.name);
    }

    #[test]
    fn test_requires_named_struct() {
        let field: syn::Field = syn::parse_quote! {()};

        let result = ParseField::from_field(&field);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"CacheDiff can only be used on structs with named fields"#
        );
    }

    #[test]
    fn test_known_attributes() {
        let parsed: KnownAttribute = syn::parse_str("rename").unwrap();
        assert_eq!(KnownAttribute::rename, parsed);

        let parsed: KnownAttribute = syn::parse_str("ignore").unwrap();
        assert_eq!(KnownAttribute::ignore, parsed);

        let parsed: KnownAttribute = syn::parse_str("display").unwrap();
        assert_eq!(KnownAttribute::display, parsed);

        let result: Result<KnownAttribute, syn::Error> = syn::parse_str("unknown");
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"Unknown cache_diff attribute: `unknown`. Must be one of `rename`, `display`, `ignore`"#
        );
    }

    #[test]
    fn test_parse_attributes() {
        let parsed: ParseAttribute = syn::parse_str(r#"rename = "Ruby version""#).unwrap();
        assert_eq!(ParseAttribute::rename("Ruby version".to_string()), parsed);

        let parsed: ParseAttribute = syn::parse_str(r#"display= my_function"#).unwrap();
        assert!(matches!(parsed, ParseAttribute::display(_)));
        let parsed: ParseAttribute = syn::parse_str(r#"ignore = "i have my reasons""#).unwrap();
        assert!(matches!(parsed, ParseAttribute::ignore(_)));

        let parsed: ParseAttribute = syn::parse_str("ignore").unwrap();
        assert!(matches!(parsed, ParseAttribute::ignore(_)));
    }
}

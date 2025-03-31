//! Shared logic for parsing
//!
//! ## Errors
//!
//! - [`ErrorBank`]: Type alias, used for error accumulation. Treat this type as a "maybe error" as it may be empty.
//!   Known errors should be converted into a [`syn::Error`] with [`combine`].
//! - [`combine`]: Converts an ErrorBank to a `Option<syn::Error>`.
//!
//! ## General Parsing
//!
//! Note: The term "attribute" here refers to a single `<k> = <v>` or `<v>` within a `#[cache_diff(...)]`. But a
//!       `syn::Attribute` refers to the entire contents on the inside of a `#[]`, for clarity I'll refer
//!       to these as either `syn::Attribute`-s or "attribute blocks."
//!
//! - [`WithSpan`]: A helper struct that annotates a parsed `T` with span information.
//! - [`parse_attrs`]: Turn `&[syn::Attribute]` into `Vec<T>`. Accumulates the first parse error per attribute block (if there is one).
//!
//! ## Strum defined enums
//!
//! These functions expect the parsed attribute to be represented as an enum that implements `strum::EnumDiscriminants`
//!
//!   - [`known_attribute`]: Parses a string into a strum discriminant, this is useful for later parsing it into the actual enum.
//!   - [`check_exclusive`]: Asserts any attributes that must be used alone are. Accumulates an error for every attribute that does not.
//!   - [`unique`]: Ensures that no attribute is repeated or accumulates all instances with repetition.

use crate::{MACRO_NAME, NAMESPACE};
use std::collections::{HashSet, VecDeque};
use std::{collections::HashMap, fmt::Display, str::FromStr};

/// Contains zero or more errors
///
/// Does not charge overdraft fees
pub(crate) type ErrorBank = VecDeque<syn::Error>;

/// Combines multiple errors into one
///
/// If no errors, returns `None` otherwise returns all errors into a single `syn::Error`.
/// As an FYI: Errors can be split later by using [syn::Error::into_iter](https://docs.rs/syn/2.0.100/syn/struct.Error.html#impl-IntoIterator-for-Error)
pub(crate) fn combine(mut errors: ErrorBank) -> Option<syn::Error> {
    if let Some(mut error) = errors.pop_front() {
        for e in errors {
            error.combine(e);
        }
        Some(error)
    } else {
        None
    }
}

/// Guarantees all attributes (`<k> = <v>` or `<v>`) are specified only once
///
/// Raises an error for each duplicated attribute `#[cache_diff(ignore, ignore)]`.
/// If no duplicate attributes returns a lookup by enum discriminant with span information.
pub(crate) fn unique<T>(
    parsed_attributes: impl IntoIterator<Item = WithSpan<T>>,
) -> Result<HashMap<T::Discriminant, WithSpan<T>>, syn::Error>
where
    T: strum::IntoDiscriminant + syn::parse::Parse,
    T::Discriminant: Eq + Display + std::hash::Hash + Copy,
{
    let mut seen = HashMap::new();
    let mut errors = ErrorBank::new();
    for attribute in parsed_attributes {
        let WithSpan(ref parsed, span) = attribute;
        let key = parsed.discriminant();
        if let Some(WithSpan(_, prior)) = seen.insert(key, attribute) {
            errors.push_back(syn::Error::new(
                span,
                format!("{MACRO_NAME} duplicate attribute: `{key}`"),
            ));
            errors.push_back(syn::Error::new(
                prior,
                format!("previously `{key}` defined here"),
            ));
        }
    }

    if let Some(error) = combine(errors) {
        Err(error)
    } else {
        Ok(seen)
    }
}

/// Check exclusive attributes
///
/// Errors if an exclusive attribute is used with any other attributes.
/// For example `ignore` would negate any other attributes so it is
/// mutually exclusive.
///
/// Does NOT check for repeated attributes for that, use [`unique`]
pub(crate) fn check_exclusive<T>(
    exclusive: T::Discriminant,
    collection: &[WithSpan<T>],
) -> Result<(), syn::Error>
where
    T: strum::IntoDiscriminant + syn::parse::Parse,
    T::Discriminant: Eq + Display + std::hash::Hash + Copy,
{
    let mut errors = ErrorBank::new();
    let mut keys = collection
        .iter()
        .map(|WithSpan(value, _)| value.discriminant())
        .collect::<HashSet<T::Discriminant>>();

    if keys.remove(&exclusive) && !keys.is_empty() {
        let other_keys = keys
            .iter()
            .map(|key| format!("`{key}`"))
            .collect::<Vec<_>>()
            .join(", ");

        for WithSpan(value, span) in collection {
            if value.discriminant() == exclusive {
                errors.push_front(syn::Error::new(
                    *span,
                    format!("cannot be used with other attributes. Remove ether `{exclusive}` or {other_keys}",),
                ))
            } else {
                errors.push_back(syn::Error::new(
                    *span,
                    format!("cannot be used with #[{NAMESPACE}({exclusive})]"),
                ))
            }
        }
    }

    if let Some(error) = combine(errors) {
        Err(error)
    } else {
        Ok(())
    }
}

/// Parses one bare word like "rename" for any iterable enum, and that's it
///
/// Won't parse an equal sign or anything else. Emits all known keys for
/// debugging help when an unknown string is passed in
pub(crate) fn known_attribute<T>(identity: &syn::Ident) -> syn::Result<T>
where
    T: FromStr + strum::IntoEnumIterator + Display,
{
    let name_str = &identity.to_string();
    T::from_str(name_str).map_err(|_| {
        syn::Error::new(
            identity.span(),
            format!(
                "Unknown {NAMESPACE} attribute: `{identity}`. Must be one of {valid_keys}",
                valid_keys = T::iter()
                    .map(|key| format!("`{key}`"))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        )
    })
}

/// Parse attributes into a vector
///
/// Returns at least one error per attribute block `#[attribute(...)]` if it cannot
/// be parsed.
pub(crate) fn parse_attrs<T>(attrs: &[syn::Attribute]) -> Result<Vec<T>, syn::Error>
where
    T: syn::parse::Parse,
{
    let mut attributes = Vec::new();
    let mut errors: VecDeque<syn::Error> = ErrorBank::new();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident(NAMESPACE)) {
        match attr
            .parse_args_with(syn::punctuated::Punctuated::<T, syn::Token![,]>::parse_terminated)
        {
            Ok(attrs) => {
                for attribute in attrs {
                    attributes.push(attribute);
                }
            }
            Err(error) => errors.push_back(error),
        }
    }

    if let Some(error) = combine(errors) {
        Err(error)
    } else {
        Ok(attributes)
    }
}

/// Helper type for parsing a type and preserving the original span
///
/// Used with [syn::punctuated::Punctuated] to capture the inner span of an attribute.
#[derive(Debug)]
pub(crate) struct WithSpan<T>(pub(crate) T, pub(crate) proc_macro2::Span);

impl<T: syn::parse::Parse> syn::parse::Parse for WithSpan<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();
        Ok(WithSpan(input.parse()?, span))
    }
}

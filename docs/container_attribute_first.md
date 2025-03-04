## Add a container attribute


Like we did with fields, we'll define an enum to hold each container attribute variant.

```rust
:::-> print.erb
<%
import = ["use std::str::FromStr;"];
import << "use strum::IntoEnumIterator;"

code = <<-EOF
/// A single attribute
#[derive(strum::EnumDiscriminants, Debug, PartialEq)]
#[strum_discriminants(derive(strum::EnumIter, strum::Display, strum::EnumString))]
#[strum_discriminants(name(KnownAttribute))]
enum ParseAttribute {
    #[allow(non_camel_case_types)]
    custom(syn::Path), // #[cache_diff(custom=<function>)]
}
EOF
%>
<%=
append(filename: "cache_diff_derive/src/parse_container.rs", use: import, code: code)
%>
```

We will go ahead and add an implementation of `syn::parse::Parse` for `KnownAttribute`, it's virtually identical:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_container.rs", code: <<-CODE)
impl syn::parse::Parse for KnownAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let identity: syn::Ident = input.parse()?;
        KnownAttribute::from_str(&identity.to_string()).map_err(|_| {
            syn::Error::new(
                identity.span(),
                format!(
                    "Unknown {NAMESPACE} attribute: `{identity}`. Must be one of {valid_keys}",
                    valid_keys = KnownAttribute::iter()
                        .map(|key| format!("`{key}`"))
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            )
        })
    }
}
CODE
%>
```

The turning an input into a vector of parsed attributes looks pretty similar as well:


```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/parse_container.rs", code: <<-CODE)
impl ParseAttribute {
    fn from_attrs(attrs: &[syn::Attribute]) -> Result<Vec<ParseAttribute>, syn::Error> {
        let mut attributes = Vec::new();
        for attr in attrs.iter().filter(|attr| attr.path().is_ident(NAMESPACE)) {
            for attribute in attr.parse_args_with(
                syn::punctuated::Punctuated::<ParseAttribute, syn::Token![,]>::parse_terminated,
            )? {
                attributes.push(attribute)
            }
        }

        Ok(attributes)
    }
}
CODE
%>
```

The actual parse code is slightly different, but it should seem like a familiar pattern:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_container.rs", code: <<-CODE)
impl syn::parse::Parse for ParseAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key: KnownAttribute = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        match key {
            KnownAttribute::custom => Ok(ParseAttribute::custom(input.parse()?)),
        }
    }
}
CODE
%>
```

Verify your intuition (and my claims) with some tests:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_container.rs", test_code: <<-CODE)
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
            ParseAttribute::from_attrs(&input.attrs)
                .unwrap()
                .first(),
            Some(ParseAttribute::custom(_))
        ));
    }
CODE
%>
```

Verify they work, and now you shouldn't see any warnings:

```
:::>> $ cargo test
:::-- $ cargo clippy
```

Now let's wire it up. Start of by adding a place to store our attribute on the container.


```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/parse_container.rs", match: /pub\(crate\) struct ParseContainer/, code: <<-CODE )
/// Container (i.e. struct Metadata { ... }) and its parsed attributes
/// i.e. `#[cache_diff( ... )]`
#[derive(Debug)]
pub(crate) struct ParseContainer {
    /// The proc-macro identifier for a container i.e. `struct Metadata { }` would be a programatic
    /// reference to `Metadata` that can be used along with `quote!` to produce code.
    pub(crate) ident: syn::Ident,
    /// An optional path to a custom diff function
    /// Set via attribute on the container i.e. `#[cache_diff(custom = <function>)]`
    pub(crate) custom: Option<syn::Path>,
    /// Fields (i.e. `name: String`) and their associated attributes i.e. `#[cache_diff(...)]`
    pub(crate) fields: Vec<ParseField>,
}
CODE
%>
```

Then update the logic for building the container:

```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/parse_container.rs", match: /pub\(crate\) fn from_derive_input/, code: <<-CODE )
impl ParseContainer {
    pub(crate) fn from_derive_input(input: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let ident = input.ident.clone();
        let attributes = ParseAttribute::from_attrs(&input.attrs)?;
        let custom = attributes
            .into_iter()
            .map(|attribute| match attribute {
                ParseAttribute::custom(path) => path,
            })
            .last();
        let fields = match input.data {
            syn::Data::Struct(syn::DataStruct {
                fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
                ..
            }) => named,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("{MACRO_NAME} can only be used on named structs"),
                ))
            }
        }
        .into_iter()
        .map(ParseField::from_field)
        .collect::<Result<Vec<ParseField>, syn::Error>>()?;

        if let Some(field) = fields
            .iter()
            .find(|field| matches!(field.ignore.as_deref(), Some("custom")))
        {
            if custom.is_none() {
                return Err(syn::Error::new(ident.span(),
                            format!(
                                "field `{field}` on {container} marked ignored as custom, but no `#[{NAMESPACE}(custom = <function>)]` found on `{container}`",
                                field = field.ident,
                                container = &ident,
                            )));
            }
        }

        if fields.iter().any(|f| f.ignore.is_none()) {
            Ok(ParseContainer {
                ident,
                fields,
                custom,
            })
        } else {
            Err(syn::Error::new(ident.span(), format!("No fields to compare for {MACRO_NAME}, ensure struct has at least one named field that isn't `{NAMESPACE}(ignore)`")))
        }
    }
}
CODE
%>
```

The are two new things here, one of them is small and expected:

```rust
let custom = attributes
    .into_iter()
    .map(|attribute| match attribute {
        ParseAttribute::custom(path) => path,
    })
    .last();
```

This is where we're pulling out the attribute information and querying it, similar to how we did it with the `ParseField`. Hopefully you expected that addition. The other is: A bunch of manual error handling. For example:

```rust
if let Some(field) = fields
    .iter()
    .find(|field| matches!(field.ignore.as_deref(), Some("custom")))
{
    if custom.is_none() {
        return Err(syn::Error::new(ident.span(),
                    format!(
                        "field `{field}` on {container} marked ignored as custom, but no `#[{NAMESPACE}(custom = <function>)]` found on `{container}`",
                        field = field.ident,
                        container = &ident,
                    )));
    }
}
```

Previously when I added the ability to set a field as ignored with a reason, it gave us the ability to add a preference signal that did something meaningful. In this case we are saying that if the user adds a `#[cache_diff(ignore = "custom")]` to one of their fields, they MUST also add a `#[cache_diff(custom = <function>)]` to the container. Because proc macros make it faster for the end user to generate and manipulate code, it makes it faster for them to make mistakes too. You could imagaine a scenario where they're playing around with configuration options and they accidentally delete the container attribute line, and it's not caught in code review and the linter isn't loud enough, so they deploy with code that looks correct but isn't. The nice thing about adding this error here, is that when the user tries to compile their code with invalid state, it's not representable and they get a clear error explaining what went wrong and how to fix it. Coming from (such a flexible and dynamic language as) Ruby, these defensive codeing practices are second nature to me. [A talk by Avdii back from 2011 comes to mind](https://www.youtube.com/watch?v=t8s2MqnDPD8). You don't need to pre-think every possible thing a coder can do wrong with your library, but it's worth both thinking about it ahead of your first proc-macro release, as well as being on the lookout for examples of incorrect usage from other devs and from your own code and notes.


The other error is here:

```rust
if fields.iter().any(|f| f.ignore.is_none()) {
    Ok(ParseContainer {
        ident,
        fields,
        custom,
    })
} else {
    Err(syn::Error::new(ident.span(), format!("No fields to compare for {MACRO_NAME}, ensure struct has at least one named field that isn't `{NAMESPACE}(ignore)`")))
}
```

If someone tries to use the macro on an empty struct or accidentally ignores all the fields, then I don't want the derive code to compile. If someone has a legitimate use for a type that is `impl CacheDiff` but always returns an empty difference set, that's fine...but I won't help them construct such an abomination (i.e. I'm not blocking them from implementing it manually, only blocking it via a derive macro). Whenever I write reflection code, I like to have a strong sense of what code paths should be encouraged, which should be allowable but discouraged, and which should be impossible. I also believe that many programmers have more smarts than empathy and thanks to Turing completeness, that means statements like "I cannot imagine a reason why anyone would want to X," may be due to lack of imagination, rather than a lack of a good reason for doing that thing.

Taking these two errors out doesn't change much, but I consider the error experience, how our interfaces behave in failure scenarios, to be a true test of quality software design. Even better design, allows us to assert those failure scenarios via tests:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_container.rs", test_code: <<-CODE)
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
CODE
%>
```

With all of this in place, it's time to put a ribbon on it and tie it all together. In the next section we'll use our newly defined field and container attributes in the proc macro output.

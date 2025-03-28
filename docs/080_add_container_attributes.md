
<span id="chapter_08" />

## 08: Add attributes to ParseContainer

We'll define an enum to hold each container attribute variant as we did with fields. Add this code:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/parse_container.rs", code: <<~CODE)
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
CODE
%>
```

We will go ahead and add an implementation of `syn::parse::Parse` for `KnownAttribute`, it's virtually identical:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_container.rs", code: <<-CODE)
impl syn::parse::Parse for KnownAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let identity: syn::Ident = input.parse()?;
        crate::shared::known_attribute(&identity)
    }
}
CODE
%>
```

Now, implement the parsing logic. Add this code:

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

Verify your intuition (and my claims) with some tests. Add this test code:

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
            crate::shared::attribute_lookup::<ParseAttribute>(&input.attrs)
                .unwrap()
                .remove(&KnownAttribute::custom)
                .unwrap()
                .into_inner(),
            ParseAttribute::custom(_)
        ));
    }
CODE
%>
```

Verify they work:

```
:::>- $ cargo test
```

Now, we can parse attributes for containers. Let's add that information to our container struct. Replace this code:

```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/parse_container.rs", match: /pub\(crate\) struct ParseContainer/, code: <<-CODE )
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
CODE
%>
```

Add import code:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/parse_container.rs", use: ["use crate::NAMESPACE;", "use crate::shared::WithSpan;", "use std::collections::VecDeque;"])
%>
```

Now, replace this code:

```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/parse_container.rs", match: /pub\(crate\) fn from_derive_input/, code: <<-CODE )
impl ParseContainer {
    pub(crate) fn from_derive_input(input: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let ident = input.ident.clone();
        let generics = input.generics.clone();
        let mut fields = Vec::new();
        let mut errors = VecDeque::new();
        let mut custom = None;

        match crate::shared::attribute_lookup::<ParseAttribute>(&input.attrs) {
            Ok(mut lookup) => {
                for (_, WithSpan(value, _)) in lookup.drain() {
                    match value {
                        ParseAttribute::custom(path) => custom = Some(path),
                    }
                }
            }
            Err(error) => errors.push_back(error),
        }

        let syn_fields = match input.data {
            syn::Data::Struct(syn::DataStruct {
                fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
                ..
            }) => named,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("{MACRO_NAME} can only be used on named structs"),
                ));
            }
        };

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
                    }
                }
                Ok(active_field) => fields.push(active_field),
                Err(error) => {
                    errors.push_back(error);
                }
            }
        }

        if let Some(mut error) = errors.pop_front() {
            for e in errors {
                error.combine(e);
            }
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
CODE
%>
```

We use the same techniques that we saw in `ParseField` to accumulate as many errors as possible. We lookup the custom attribute from our `HashMap` similar to what we did with `ParseField`:

```rust
:::-> $ grep -A1000 'match attribute_lookup' cache_diff_derive/src/parse_container.rs | awk '/^        }/ {print; exit} {print}'
```




```rust
:::-> $ grep -A1000 'if let Some(field) = fields' cache_diff_derive/src/parse_container.rs | awk '/\;/ {print; exit} {print}'
```

Previously, when I added the ability to set a field as ignored with a reason, it allowed us to add a preference signal that did something meaningful. In this case, we are saying that if the user adds a `#[cache_diff(ignore = "custom")]` to one of their fields, they MUST also add a `#[cache_diff(custom = <function>)]` to the container. Because proc macros make it faster for the end user to generate and manipulate code, it makes it faster for them to make mistakes, too. You could imagine a scenario where they're playing around with configuration options, and they accidentally delete the container attribute line, and it's not caught in code review, and the linter isn't loud enough, so they deploy with code that looks correct but isn't.

The nice thing about adding this error here is that when the user tries to compile their code with an invalid state, it's not representable, and they get a clear error explaining what went wrong and how to fix it. Coming from (such a flexible and dynamic language as) Ruby, these defensive coding practices are second nature to me. [A talk by Avdii back from 2011 comes to mind as having some good examples](https://www.youtube.com/watch?v=t8s2MqnDPD8). You don't need to pre-think every possible thing a coder can do wrong with your library, but it's worth spending a moment or two thinking of failure modes ahead of your first proc-macro release. Once you've released your macro, be on the lookout for examples of incorrect usage from other devs and from your code and notes.

The other error is here:

```rust
:::-> $ grep -A1000 'if fields.iter().any(|f| f.ignore.is_none())' cache_diff_derive/src/shared.rs | awk '/        }/ {print; exit} {print}'
```

I don't want the derived code to compile if someone tries to use the macro on an empty struct or accidentally ignores all the fields. If someone has a legitimate use for a type that is `impl CacheDiff` but always returns an empty difference set, that's fine...but I won't help them construct such an abomination. I'm not blocking them from implementing it manually; I'm only blocking it via a derive macro. Whenever I write reflection code, I like to have a strong sense of what code paths should be encouraged, which should be allowable but discouraged, and which should be impossible. I also believe that many programmers have more smarts than empathy, and thanks to Turing completeness, that means statements like "I cannot imagine a reason why anyone would want to X" may be due to a lack of imagination rather than a lack of a good reason for doing that thing.

I consider the error experience, how our interfaces behave in failure scenarios, to be a true test of quality software design. Even better design, allows us to assert those failure scenarios via tests. Add the testing code now:

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

With all this in place, it's time to Derive to the finish line. The following section will use our newly defined field and container attributes in the proc macro output.

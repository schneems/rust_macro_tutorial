## Add field attributes

Our macro will need both field and container attributes. You may recall, that our readme driven development left us with three things to customize on the field:

- [cache_diff(ignore)](https://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#ignore-attributes)
- [cache_diff(display = <code path>)](https://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#handle-structs-missing-display)
- [cache_diff(rename = "<new name>")](https://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#rename-attributes)

And one thing to customize on the container:

- [Customize cache behavior for some fields without having to manually implement the trait for the rest. Container attribute: `cache_diff(custom = <code path>)`](hhttps://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#custom-logic-for-one-field-example)

Initial prototyping suggested that it was useful for developers to list why a certain field was ignored, so beyond a simple boolean flag for ignore, I decided that `cache_diff(ignore = "Reason why field is ignored")` should also be valid. In the real code I'm special casing `ignore = "custom"` to trigger an additional check.

Like before, we'll represent this state in code and fill out the rest of our program to be capable of generating that code. We will represent individual attributes as an enum, and use the [strum](https://crates.io/crates/strum) crate to make parsing and error generation easier:

```
:::>- $ cargo add strum@0.26 --package cache_diff_derive --features derive
```

Now define the enum:

```rust
:::>> print.erb
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
    rename(String), // #[cache_diff(rename="...")]
    #[allow(non_camel_case_types)]
    display(syn::Path), // #[cache_diff(display=<function>)]
    #[allow(non_camel_case_types)]
    ignore(String), // #[cache_diff(ignore)]
}
EOF
%>
<%=
append(filename: "cache_diff_derive/src/parse_field.rs", use: import, code: code)
%>
```

In addition to enum variants such as `ParseAttribute::rename(...)`, the strum crate is creating a "discriminant" enum that has the same named variants, but without any inputs. We're telling strum to name this enum `KnownAttribute` and give it the ability to iterate over all its variants (`strum::EnumIter`), print the name of each variant (`strum::Display`), and generate a variant from a string (`strum::EnumString`).

We will implement the `syn::parse::Parse` trait to allow syn to parse a stream of tokens into our data structures. We'll start off with our `KnownAttribute` enum:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_field.rs", use: "use crate::NAMESPACE;", code: <<-CODE)
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

Here's how that translates to code:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_field.rs", test_code: <<-CODE)
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
CODE
%>
```

We can now parse individual keywords such as `rename` into a `KnownAttribute` enum. We can use this to generate an implementation of `syn::parse::Parse` for our `ParseAttribute` which uses a `<key> = <value>` structure:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_field.rs", code: <<-CODE)
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
CODE
%>
```

Because we previously implemented `syn::parse::Parse` for `KnownAttribute`, we can parse the input and then match against the enum. When a `syn::parse::ParseStream` is parsed successfully, then that part of the stream is consumed. That means that in the case of `rename` and `display` we require that the user gives us an equal sign next. This can be parsed using `syn::Token![=]`:

```rust
input.parse::<syn::Token![=]>()?;
```

> Protip: If the whole `syn::parse::ParseStream` isn't consumed in the body of a parse implementation, then it will error. This would prevent `KnownAttribute` from accidentally parsing an input like `rename = "true"` as it would match the first ident `rename` but wouldn't consume the rest. The error from syn when this happend isn't very intuititve, so if you are puzzling why your parse invocation fails, make sure you've consumed everything. You can use `eprintln` to debug.

Once the equal sign is parsed, then we need to parse the value. A `syn::LitStr` will capture a string with quotes. So for an input of `rename = "Ruby VERSION"` the `syn::LitStr` will capture `"Ruby VERSION"`. We can extract a string from it by calling the `.value()` associated function:

```rust
Ok(ParseAttribute::rename(
    input.parse::<syn::LitStr>()?.value(),
))
```

The `display` attribute holds a `syn::Path` which is described as "A path at which a named item is exported." This allows us to accept a function like `my_display` or a fully qualified path to a function like: `std::path::PathBuf::display`. Because this is a syn type, we can parse directly into it without needing any type annotations:

```rust
Ok(ParseAttribute::display(input.parse()?))
```

Finally, the `ignore` attribute can be used in one of two ways either `cache_diff(ignore)` or `cache_diff(ignore = "reason")`. To handle these two scenarios, we can inspect the `ParseStream` via `peek()` to see if it contains an equal or not. If it does, parse it and expect a literal string, if not, we'll default to some internal value. So far, the value is only used as a marker in the source to future developers for why it was ignored, so we could chose any default string, I picked "no reason given":


```rust
KnownAttribute::ignore => {
    if input.peek(syn::Token![=]) {
        input.parse::<syn::Token![=]>()?;
        Ok(ParseAttribute::ignore(
            input.parse::<syn::LitStr>()?.value(),
        ))
    } else {
        Ok(ParseAttribute::ignore("no reason given".to_string()))
    }
}
```

With all that in place, you can add a test and validate that we can parse it into our data structure:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_field.rs", test_code: <<-CODE)
    #[test]
    fn test_parse_attributes() {
        let parsed: ParseAttribute = syn::parse_str(r#"rename = "Ruby version""#).unwrap();
        assert_eq!(ParseAttribute::rename("Ruby version".to_string()), parsed);

        let parsed: ParseAttribute = syn::parse_str(r#"display= my_function"#).unwrap();
        assert!(matches!(parsed, ParseAttribute::display(_))); let parsed: ParseAttribute = syn::parse_str(r#"ignore = "i have my reasons""#).unwrap();
        assert!(matches!(parsed, ParseAttribute::ignore(_)));

        let parsed: ParseAttribute = syn::parse_str("ignore").unwrap();
        assert!(matches!(parsed, ParseAttribute::ignore(_)));
    }
CODE
%>
```

So far, so good, but if you tried to parse multiple attributes then you'll get a failure, We need to be able to parse a comma deleniated set of attributes. And this is how I choose to implement that:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/parse_field.rs", code: <<-CODE)
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

Here's a quick test before I circle back and explain what's going on:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/parse_field.rs", test_code: <<-CODE)
    #[test]
    fn test_parse_rename_ignore_attribute() {
        let field: syn::Field = syn::parse_quote! {
            #[cache_diff(rename="Ruby version", ignore)]
            name: String
        };

        assert_eq!(
            vec![
                ParseAttribute::rename("Ruby version".to_string()),
                ParseAttribute::ignore("default".to_string()),
            ],
            ParseAttribute::from_attrs(&field.attrs).unwrap()
        );
    }
CODE
%>
```

This code pulls out attributes on a field and iterates over them. The `syn` code receives ALL attributes, so we have to filter by our macro's namespace, or else we'll accidentally try to parse things like `serde(...)` attributes from other macros. This will yield a `syn::Attribute`:

```rust
for attr in field
    .attrs
    .iter()
    .filter(|attr| attr.path().is_ident(NAMESPACE))
{
    // ... attr is a syn::Attribute
}
```

We use the [`syn::Attribute::parse_args_with`](https://docs.rs/syn/latest/syn/struct.Attribute.html#method.parse_args_with) function which takes a parser. We've implemented two parsers so far `KnownAttribute` and `ParseAttribute`. But we need something that can handle a comma separated set of attributes, so we turn to the pre-built `syn::punctuated::Punctuated` parser, which is actually a parser combinator, meaning it takes in other parsers as it's input. In our case we're telling it to build a set of `ParseAttribute` structs, and use commas to separate them. We then call `parse_terminated` on this parser combinator which returns an iterator of item type `ParseAttribute` that we can use to build and return our `Vec<ParseAttribute>`:

```rust
for attribute in attr.parse_args_with(
    syn::punctuated::Punctuated::<ParseAttribute, syn::Token![,]>::parse_terminated,
)? {
    attributes.push(attribute)
}
```

At this point we've added the ability to extract any cache_diff attributes from a `syn::Field` as a `Vec<ParseAttribute>`, but so far, nothing uses `ParseAttribute` outside of this module. We need to take this information and put it into a `ParseField` to make it useful. Replace this code:

```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/parse_field.rs", match: /pub\(crate\) struct ParseField/, code: <<-CODE )
/// Field (i.e. `name: String`) of a container (struct) and its parsed attributes
/// i.e. `#[cache_diff(rename = "Ruby version")]`
#[derive(Debug)]
pub(crate) struct ParseField {
    /// The proc-macro identifier for a field i.e. `name: String` would be a programatic
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
CODE
%>
```

We were already storing the ident and desired name of our field, but now we're also capturing if it was ignored or not as well as what function to use for it's display. Since this last value, `display`, isn't optional, we'll need to set it for every field. To help with this, there's a nifty utility function that returns whatever is passed to it we can use as a default [`std::convert::identity`](https://doc.rust-lang.org/std/convert/fn.identity.html). And while we're picking out sensible defaults, if we can detect that a type is a `std::path::PathBuf` then we can go ahead and default to `std::path::Path::display` since we know it does not implement `Display`. To help that detection, add a helper function:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_field.rs", code: <<-CODE)
fn is_pathbuf(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "PathBuf" && segment.arguments == syn::PathArguments::None;
        }
    }
    false
}
CODE
%>
```

This code takes in a `syn::Type`, checks if it's a path to a type and if it is and matches `PathBuf` then it returns true. Perhaps there's a more robust way to do check, if you know one...let me know.

With that helper code in place we can now extract values to build our new `ParseField`. Replace this code:


```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/parse_field.rs", match: /impl ParseField {/, code: <<-CODE )
impl ParseField {
    pub(crate) fn from_field(field: &syn::Field) -> Result<Self, syn::Error> {
        let ident = field.ident.clone().ok_or_else(|| {
            syn::Error::new(
                field.span(),
                format!("{MACRO_NAME} can only be used on structs with named fields"),
            )
        })?;

        let attributes = ParseAttribute::from_attrs(&field.attrs)?;
        let name = attributes
            .iter()
            .filter_map(|attribute| match attribute {
                ParseAttribute::rename(name) => Some(name.to_owned()),
                _ => None,
            })
            .last()
            .unwrap_or_else(|| ident.to_string().replace("_", " "));
        let display = attributes
            .iter()
            .filter_map(|attribute| match attribute {
                ParseAttribute::display(display_fn) => Some(display_fn.to_owned()),
                _ => None,
            })
            .last()
            .unwrap_or_else(|| {
                if is_pathbuf(&field.ty) {
                    syn::parse_str("std::path::Path::display")
                        .expect("PathBuf::display parses as a syn::Path")
                } else {
                    syn::parse_str("std::convert::identity")
                        .expect("std::convert::identity parses as a syn::Path")
                }
            });

        let ignore = attributes
            .into_iter()
            .filter_map(|attribute| match attribute {
                ParseAttribute::ignore(reason) => Some(reason),
                _ => None,
            })
            .last();

        Ok(ParseField {
            ident,
            name,
            ignore,
            display,
        })
    }
}

CODE
%>
```

The code starts off in a similar fashion, but then diverges by building a Vec of `ParseAttributes` that we can iterate through. An eagle-eyed reader might notice that there's nothing preventing two or three attribute declarations via our user like:

```rust
struct Metadata {
    #[cache_diff(rename = "Foo")]
    #[cache_diff(rename = "Bar")]
    #[cache_diff(rename = "Baz", rename = "Ruby Version")]
    version: String
}
```

If I wanted to be more strict, I could raise a `syn::Error` when an attribute is specified more than once, but in our case, it's simple enough that we'll say that the last one wins. So the above example would be renamed to `"Ruby Version"` and all those other attributes would be no-ops.

Each attribute we support is queried, if it doesn't exist then we set a default and keep going until all information needed to build the struct is present.

It might seem like we added a lot of code, but most of this boils down to:

- Define all valid attributes in a `ParseAttribute` enum with a `KnownAttribute` discriminant
- Implement `syn::parse::Parse` for these enums
- Implement a function that takes in a `syn::Field` and returns `Vec<ParseAttribute>`
- Add any new fields needed to your `ParseField` struct
- Adjust your building functions to use the new attribute information collected.

Sometimes it's easier to  go the other way, by defining the fields you need to for `ParseField` and then figuring out the API you want to make to support it, but from a testing perspective, it's easier to start with smaller parsers and gradually combine them to build bigger ones.


Verify tests are all passing:

```
:::>- $ cargo test
```

If your project is failing or if the tests you added didn't run, here's the full project for reference:

<details>
  <summary>Full project</summary>

```
:::>> $ exa --tree --git-ignore .
:::>> $ cat Cargo.toml
:::>> $ cat cache_diff/Cargo.toml
:::>> $ cat cache_diff_derive/Cargo.toml
:::>> $ cat cache_diff/src/lib.rs
:::>> $ cat cache_diff_derive/src/lib.rs
:::>> $ cat cache_diff_derive/src/parse_field.rs
:::>> $ cat cache_diff_derive/src/parse_container.rs
```
</details>

We're done with the field modifications. We need to adjust our container code to add a container attribute, and then our final `lib.rs` code needs to use all this tasty info we just added.


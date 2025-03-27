
<span id="chapter_07" />

## 07: Add attributes to ParseField

Our macro will need both field and container attributes. Our readme-driven development left us with three things to customize on the field:

- [`cache_diff(ignore)`](https://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#ignore-attributes)
- [`cache_diff(display = <code path>)`](https://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#handle-structs-missing-display)
- [`cache_diff(rename = "<new name>")`](https://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#rename-attributes)

And one thing to customize on the container:

- [Customize cache behavior for some fields without manually implementing the trait for the rest. Container attribute: `cache_diff(custom = <code path>)`](hhttps://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#custom-logic-for-one-field-example)

Initial prototyping suggested that it was helpful for developers to list why a specific field was ignored, so beyond a simple boolean flag for ignore, I decided that `cache_diff(ignore = "Reason why the field is ignored")` should also be valid.

Like before, we'll represent this state in code and fill out the rest of our program to be capable of generating that code. We will represent individual attributes as an enum and use the [strum](https://crates.io/crates/strum) crate to simplify parsing and error generation. Add that dependency now:

```
:::>> print.text $ cargo add strum@0.27.1 --package cache_diff_derive --features derive
:::-- $ cargo add strum@0.27.1 --package cache_diff_derive --features derive --offline
```

The dependencies look like this:

```
:::>> $ cat cache_diff_derive/Cargo.toml
```

Now, define an enum that will hold each of our attribute variants. Add this code:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/parse_field.rs", code: <<~CODE)
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
CODE
%>
```

In addition to enum variants such as `ParseAttribute::rename(...)`, the strum crate creates a "discriminant" enum with the same named variants but without input values. That means that `ParseAttribute::rename(String)` will have a corresponding `KnownAttribute::rename` (without the `String` inner value).

We're using `strum_discriminants` attribute to tell strum to name this "discriminant" enum `KnownAttribute` and give it the ability to iterate over all its variants (`strum::EnumIter`), print the name of each variant (`strum::Display`), and generate a variant from a string (`strum::EnumString`).

Attributes parse logic is similar so that we can reuse some of the logic in our field and container parsing. Create a new file and add this code:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/shared.rs", use: [
    "use crate::{MACRO_NAME, NAMESPACE};",
    "use std::{collections::HashMap, fmt::Display, str::FromStr};"
    ], code: <<-CODE)
/// Parses one bare word like "rename" for any iterable enum, and that's it
///
/// Won't parse an equal sign or anything else
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
CODE
%>
```

This code takes in a `syn::Ident` described in the docs as "A word of Rust code, which may be a keyword or legal variable name." This `syn::Ident` struct holds any single bare word like `struct` or `let` or anything that could be a valid variable name like `rename`. The return result will be the discriminant `KnownAttribute` we defined earlier. However, since it is generic, we can reuse it to produce any type with the same behaviors (traits). The traits on it tell us that we must be able to construct this value from a str (`FromStr`). It must be an iterable enum (`strum::IntoEnumIterator`). And it needs to be a thing we can show our end user (`Display`).

The body extracts a string from our bare word and uses the `FromStr` trait to try to produce an enum fallibly. If it fails, we emit a nice parse error explaining what values are valid so the user doesn't have to stop and look up our docs.

Make sure your project knows about this new code by adding this file:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/lib.rs", mod: "mod shared;") %>
```

We will implement the `syn::parse::Parse` trait to allow syn to parse a stream of tokens into our data structures. We'll start with our `KnownAttribute` enum. Add this code:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_field.rs", code: <<-CODE)
impl syn::parse::Parse for KnownAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let identity = input.parse::<syn::Ident>()?;
        crate::shared::known_attribute(&identity)
    }
}
CODE
%>
```

This code takes in a `syn::parse::ParseStream`. From the docs on that module:

```
> Parsing interface for parsing a token stream into a syntax tree node.
>
> Parsing in Syn is built on parser functions that take in a [`ParseStream`]
> and produce a [`Result<T>`] where `T` is some syntax tree node.
```

So, a `ParseStream` could represent tokens in valid or invalid rust code. It has a `parse` function on it, and we can use that function to parse into any `T` that implements `syn::parse::ParseStream` such as `syn::Ident`, which we are doing here. We pass that ident into the `crate::shared::known_attribute` function we just defined.

To see it in action, add a test now:

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

This test uses `syn::parse_str` to take in single keywords we defined, such as "rename," and convert them into a `KnownAttribute` enum variant. The syn crate can do this because we implemented `syn::parse::Parse` on this `KnownAttribute` enum. You can also see our nice error message when we try to parse `unknown`. There is no discriminant `KnownAttribute::unknown` because there is no `ParseAttribute::unknown(...)` variant.

In addition to errors we manually added, there are other ways for parsing to fail:

```rust
:::>> print.erb
    #[test]
    fn test_other_error_cases() {
        let result: Result<KnownAttribute, syn::Error> = syn::parse_str("'iamnotanident'");
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"cannot parse string into token stream"#
        );

        let result: Result<KnownAttribute, syn::Error> = syn::parse_str("rename and more!");
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"unexpected token"#
        );
    }
CODE
%>
```

The first error is because `input.parse::<syn::Ident>()?` is expecting a bare word, not a string, so it errors. The second error happens because we passed in more tokens than our parser could use. It could parse `rename` and turn it into a `KnownAttribute::rename` variant, but after that, there's still `and more !` left in the parse stream.

Syn requires that all tokens are consumed in the body of a `syn::parse::Parse` implementation, or it will raise an error. This behavior prevents accidentally taking in input that we're not expecting. Unfortunately, the error message isn't clear in the test context. If you get "unexpected token" when you're prototyping your macro, you'll need to add some `eprintln!` calls to understand what's happening inside your parse implementation.

We will use this capability to generate an implementation of `syn::parse::Parse` for our `ParseAttribute` to allow it to handle a `<key> = <value>` structure. Add this code:

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

Because we previously implemented `syn::parse::Parse` for `KnownAttribute`, we can parse the input and then match against the enum. When a `syn::parse::ParseStream` is parsed successfully, then that part of the stream is consumed. That means that in the case of `rename` and `display`, we require that the user gives us an equal sign next. This can be parsed using `syn::Token![=]`:

```rust
input.parse::<syn::Token![=]>()?;
```

After the equal sign is parsed, parse the value. A `syn::LitStr` struct can hold a string with quotes. For an input of `rename = "Ruby VERSION"` the `syn::LitStr` will capture `"Ruby VERSION"`. We can extract a string from a `syn::LitStr` by calling the `.value()` associated function:

```rust
Ok(ParseAttribute::rename(
    input.parse::<syn::LitStr>()?.value(),
))
```

The `display` attribute holds a `syn::Path`, which is described as "A path at which a named item is exported." This type allows us to accept a function like `my_display` or a fully qualified path to a function like: `std::path::PathBuf::display`. Because this is a syn type, we can parse directly into it without needing any type annotations:

```rust
Ok(ParseAttribute::display(input.parse()?))
```

Finally, the `ignore` attribute can be used in one of two ways: either `cache_diff(ignore)` or `cache_diff(ignore = "reason")`. To handle these two scenarios, we can inspect the `ParseStream` via `peek()` to see if it contains an equal. If it does, parse it and expect a literal string afterward. If not, we'll default to some internal value. So far, the value is only used as a marker in the source for future developers to explain why it was ignored so we could choose any default string. I picked "no reason given":


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

With all that in place, you can add a test and validate that we can parse it into our data structure. Add this code:

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

So far, so good, but if you tried to parse multiple attributes like `rename = "ruby version", display = my_function`, then you'll get a failure. Why? Because `ParseAttribute` only parses a single attribute at a time. It would consume `rename = "ruby version"` but leave `, display = my_function` untouched.

To parse a comma-separated set of attributes, add this code:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/shared.rs", use: "use std::collections::VecDeque;", code: <<-CODE)
fn parse_attrs<T>(attrs: &[syn::Attribute]) -> Result<Vec<T>, syn::Error>
where
    T: syn::parse::Parse,
{
    let mut attributes = Vec::new();
    let mut errors = VecDeque::new();
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

    if let Some(mut error) = errors.pop_front() {
        for e in errors {
            error.combine(e);
        }
        Err(error)
    } else {
        Ok(attributes)
    }
}
CODE
%>
```

This code pulls out attributes on a field and iterates over them. The `syn` code receives ALL attributes, so we have to filter by our macro's namespace, or else we'll accidentally try to parse things like `serde(...)` attributes from other macros. This loop will yield a `syn::Attribute`:

```rust
:::-> $ grep -A1000 'for attr in attrs' cache_diff_derive/src/shared.rs | awk '/{/ {print; exit} {print}'
:::>> print.text // ... attr is a syn::Attribute
```

We use the [`syn::Attribute::parse_args_with`](https://docs.rs/syn/latest/syn/struct.Attribute.html#method.parse_args_with) function, which takes a parser. We've implemented two parsers: `KnownAttribute` and `ParseAttribute`. But we need something that can handle a comma-separated set of attributes, so we turn to the pre-built `syn::punctuated::Punctuated` parser, which is a parser combinator, meaning it takes in other parsers as its input. In our case, we're telling it to build a set of `ParseAttribute` enums and use commas (`syn::Token![,]`) to separate them. We then call `parse_terminated` on this parser combinator, which returns an iterator of item type `ParseAttribute` that we can use to build and return our `Vec<ParseAttribute>`:

```rust
:::-> $ grep -A1000 'attr.parse_args_with' cache_diff_derive/src/shared.rs | awk '/}/ {print; exit} {print}'
```

And add a test for the behavior:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/shared.rs", test_use: "    use super::*;", test_code: <<-CODE)
    #[test]
    fn test_parse_attrs_vec_demo() {
        let field: syn::Field = syn::parse_quote! {
            #[cache_diff("Ruby version")]
            name: String
        };

        assert_eq!(
            vec![syn::parse_str::<syn::LitStr>(r#""Ruby version""#).unwrap()],
            parse_attrs::<syn::LitStr>(&field.attrs).unwrap()
        );
    }
CODE
%>
```

While our `T` will eventually be `ParseAttribute`, this unit test exercises the behavior using another struct that implements `syn::parse::Parse` that we saw earlier, `syn::LitStr`. This test represents syntax that is not valid in our final macro (we don't accept a string literal "Ruby version" without any `<key> =` before it). This test shows how this function can be used as a building block with any struct that implements `syn::parse::Parse`.

At this point, we've added the ability to extract any cache_diff attributes from a `syn::Field` as a `Vec<T>` such as `Vec<ParseAttribute>`, but there's nothing stopping someone from accidentally using a duplicate configuration such as `cache_diff(rename = "Ruby", rename = "Rust")`. To prevent that and raise nice errors, we can use a different iterator that guarantees all entries are unique [`std::collections::HashMap](https://doc.rust-lang.org/std/collections/struct.HashMap.html) (rather than a `Vec` which can hold repeated entries). The discriminant of our attributes `KnownAttribute` already implements `Hash`, so we can use this as a key. The value will hold a `ParseAttribute` and span information to pinpoint precisely where an attribute was duplicated in the code. For that span information, create a helper struct.

Add this code:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/shared.rs", code: <<-CODE)
/// Helper type for parsing a type and preserving the original span
///
/// Used with [syn::punctuated::Punctuated] to capture the inner span of an attribute.
#[derive(Debug)]
pub(crate) struct WithSpan<T>(pub(crate) T, pub(crate) proc_macro2::Span);

impl<T> WithSpan<T> {
    #[cfg(test)]
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

impl<T: syn::parse::Parse> syn::parse::Parse for WithSpan<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();
        Ok(WithSpan(input.parse()?, span))
    }
}
CODE
%>
```

This new `WithSpan` struct can hold any `impl syn::parse::Parse` value, such as `ParseAttribute` or a `syn::LitStr`, as we saw in the tests before. Verify that behavior by adding a test:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/shared.rs", test_code: <<-CODE)
    #[test]
    fn test_parse_attrs_with_span_vec_demo() {
        let field: syn::Field = syn::parse_quote! {
            #[cache_diff("Ruby version")]
            name: String
        };

        assert_eq!(
            &syn::parse_str::<syn::LitStr>(r#""Ruby version""#).unwrap(),
            parse_attrs::<WithSpan<syn::LitStr>>(&field.attrs)
                .unwrap()
                .into_iter()
                .map(WithSpan::into_inner)
                .collect::<Vec<_>>()
                .first()
                .unwrap()
        );
    }
CODE
%>
```

You might notice that we're not returning from a failure early in our code, instead we're building up a `VecDeque<syn::Error>` and using that to combine multiple errors into one before returning. While programmers of dynamic languages like Python and Ruby are used to fixing an error only to see a brand new error, Rust developers expect the compiler to show them as many problems as possible at once. This error accumulation approach allows us to emit multiple errors from multiple attribute lines. For example:

```
:::-> file.write cache_diff/tests/fails/multiple_unknown.stderr
error: Unknown cache_diff attribute: `unknown`. Must be one of `rename`, `display`, `ignore`
 --> tests/fails/multiple_unknown.rs:5:18
  |
5 |     #[cache_diff(unknown)]
  |                  ^^^^^^^

error: Unknown cache_diff attribute: `unknown`. Must be one of `rename`, `display`, `ignore`
 --> tests/fails/multiple_unknown.rs:6:18
  |
6 |     #[cache_diff(unknown = "value")]
  |                  ^^^^^^^

error: Unknown cache_diff attribute: `unknown`. Must be one of `rename`, `display`, `ignore`
 --> tests/fails/multiple_unknown.rs:7:18
  |
7 |     #[cache_diff(unknown = function)]
  |                  ^^^^^^^
```

Now, we have the building blocks for a generic function with the properties we want. Add this code now:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/shared.rs", code: <<-CODE)
/// Parses all attributes and returns a lookup with the parsed value and span information where it was found
///
/// - Guarantees attributes are not duplicated
pub(crate) fn attribute_lookup<T>(
    attrs: &[syn::Attribute],
) -> Result<HashMap<T::Discriminant, WithSpan<T>>, syn::Error>
where
    T: strum::IntoDiscriminant + syn::parse::Parse,
    T::Discriminant: Eq + Display + std::hash::Hash + Copy,
{
    let mut seen = HashMap::new();
    let mut errors = VecDeque::new();
    let parsed_attributes = parse_attrs::<WithSpan<T>>(attrs)?;
    for attribute in parsed_attributes {
        let WithSpan(ref parsed, span) = attribute;
        let key = parsed.discriminant();
        if let Some(WithSpan(_, prior)) = seen.insert(key, attribute) {
            errors.push_back(
                syn::Error::new(
                    span,
                    format!("{MACRO_NAME} duplicate attribute: `{key}`")
                )
            );
            errors.push_back(
                syn::Error::new(
                    prior,
                    format!("previously `{key}` defined here"),
                )
            );
        }
    }

    if let Some(mut error) = errors.pop_front() {
        for e in errors {
            error.combine(e);
        }
        Err(error)
    } else {
        Ok(seen)
    }
}
CODE
%>
```

This function signature enforces the behavior we want via types and traits:

```rust
:::-> $ grep -A1000 'pub(crate) fn attribute_lookup' cache_diff_derive/src/shared.rs | awk '/^{/ {print; exit} {print}'
```

Like `parse_attrs`, it takes in the `&[syn::Attribute]` slice from the `syn::Field`, but the return value enforces that `T` (such as the enum `ParseAttribute`) must implement `strum::IntoDiscriminant` and `syn::parse::Parse` and that the key must be a discriminant of `T`. Storing that in a `HashMap` guarantees we will have at most one `KnownAttribute`, which maps to one `WithSpan<ParseAttribute>`. I returned the `WithSpan<T>` instead of `T` as this function only enforces that attributes are unique. In the future, an author may want to add additional constraints, such as raising an error when `ignore` is used with an attribute such as `rename`, which would imply a mistake by the implementer as `rename` would have no effect due to `ignore`.

Inside the function, a vec of all attributes is built and iterated over:

```rust
:::-> $ grep -A1000 '    let parsed_attributes =' cache_diff_derive/src/shared.rs | awk '/        let WithSpan/ {print; exit} {print}'
```

We try inserting the attribute into the HashMap based on the discriminant key:

```rust
:::-> $ grep -A1000 '        if let Some(WithSpan' cache_diff_derive/src/shared.rs | awk '/        }/ {print; exit} {print}'
```

If a prior entry exists, it represents an error, as each attribute should only have one representation. The first `syn::Error` added points at the most recent attribute we tried to add, while the second error added points at the attribute that was already in the HashMap. The result will look something like:

```
:::-> file.write cache_diff/tests/fails/duplicate_attribute.stderr
error: CacheDiff duplicate attribute: `rename`
 --> tests/fails/duplicate_attribute.rs:5:34
  |
5 |     #[cache_diff(rename = "foo", rename = "bar")]
  |                                  ^^^^^^

error: previously `rename` defined here
 --> tests/fails/duplicate_attribute.rs:5:18
  |
5 |     #[cache_diff(rename = "foo", rename = "bar")]
  |                  ^^^^^^
```

At this point, we've added the ability to extract any cache_diff attributes from a `syn::Field` as a `HashMap<KnownAttribute, WithSpan<ParseAttribute>>`, but so far, nothing uses `ParseAttribute`. We need to put this information into a `ParseField` to make it useful. Replace this code:

```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/parse_field.rs", match: /pub\(crate\) struct ParseField/, code: <<-CODE )
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
CODE
%>
```

We were already storing the ID and desired name of our field, but now we're also capturing whether it was ignored and what function to use for its display.

Since this last field, `display`, isn't optional, we'll need to set it for every `ParseField`. But how can we do that since `display` is optional? To help with this problem, a nifty utility function [`std::convert::identity`](https://doc.rust-lang.org/std/convert/fn.identity.html) that returns whatever is passed to it, we will use that as a default. And while we're picking out sensible defaults, if we can detect that a type is a `std::path::PathBuf`, then we can go ahead and default to `std::path::PathBuf::display` since we know paths do not implement `Display` by default.

Add this helper function now:

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

This code takes in a `syn::Type` and checks if it's a path to a type. If it is and matches `PathBuf`, then it returns true. Perhaps there's a more robust way to check; if you know one, let me know. With that helper code in place, we can now extract values to build our new `ParseField`.

Import the helper struct:

```rust
:::>> print.erb
<%=
append(filename: "cache_diff_derive/src/parse_field.rs", use: "use crate::shared::WithSpan;")
%>
```

Replace this code:

```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/parse_field.rs", match: /impl ParseField {/, code: <<-CODE )
impl ParseField {
    pub(crate) fn from_field(field: &syn::Field) -> Result<Self, syn::Error> {
        let mut rename = None;
        let mut ignore = None;
        let mut display = None;
        let ident = field.ident.clone().ok_or_else(|| {
            syn::Error::new(
                field.span(),
                format!("{MACRO_NAME} can only be used on structs with named fields"),
            )
        })?;

        for (_, WithSpan(attribute, span)) in
            crate::shared::attribute_lookup::<ParseAttribute>(&field.attrs)?.drain()
        {
            match attribute {
                ParseAttribute::rename(inner) => rename = Some(inner),
                ParseAttribute::ignore(inner) => ignore = Some((inner, span)),
                ParseAttribute::display(inner) => display = Some(inner),
            }
        }

        if let Some((_, span)) = ignore {
            if display.is_some() || rename.is_some() {
                return Err(syn::Error::new(
                        span,
                        format!("The cache_diff attribute `{}` renders other attributes inactive, remove additional attributes", KnownAttribute::ignore)
                    )
                );
            }
        }

        let name = rename
            .unwrap_or_else(|| ident.to_string().replace("_", " "));
        let display = display
            .unwrap_or_else(|| {
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
CODE
%>
```

This code iterates over all parsed attributes to pull out values we care about from the `HashMap` via `drain()`:

```rust
:::-> $ grep -A1000 'for \(_, WithSpan\(attribute, span\)\) in' cache_diff_derive/src/shared.rs | awk '/^        }/ {print; exit} {print}'
```

From there we check for possible developer mistakes by raising an error if `ignore` is detected with either `display` or `rename`:

```rust
:::-> $ grep -A1000 'if let Some\(\(_, span\)\) = ignore' cache_diff_derive/src/shared.rs | awk '/^        }/ {print; exit} {print}'
```

Then default values are defined and a `ParseField` is returned.

We added a lot of code making many small steps. This process  boils down to:

- Define all valid attributes in a `ParseAttribute` enum with a `KnownAttribute` discriminant
- Implement `syn::parse::Parse` for these enums
- Implement a function that takes in a `&[syn::Attribute]` and returns `HashMap` that allows us to pull out the `ParseAttribute`
- Add any new fields needed to your `ParseField` struct
- Adjust your building functions to use the new attribute information collected.

We're done with the field modifications but haven't implemented the logic in our primary derive function yet. We will do that shortly. We also haven't added a test for this new syntax.

Previously we used integration tests in the form of doctests, however I want the ability to assert failing behavior, such as an attribute that's defined twice, and I want to assert that we're pointing at the spans we expect. To do that, we will add the [`try_build`](https://crates.io/crates/trybuild) crate, which can help us visualize compiler errors that our users will see.

```
:::>> $ cargo add --dev trybuild@1.0.104 --package cache_diff
```

Now use this library to assert that all fixtures in `test/fails` fail to compile and `tests/pass` successfully compile.

```rust
:::>> file.write cache_diff/tests/compilation_tests.rs
#[test]
fn should_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fails/*.rs");
}

#[test]
fn should_compile() {
    let t = trybuild::TestCases::new();
    t.pass("tests/pass/*.rs");
}
```

Now add a compilation failure case.

```rust
:::>> file.write cache_diff/tests/fails/duplicate_attribute.rs
use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct CustomDiffFn {
    #[cache_diff(rename = "foo", rename = "bar")]
    name: String,
}

fn main() {}
```

And assert the output of that failure case:

```
:::>> $ cat cache_diff/tests/fails/duplicate_attribute.stderr
```

We also want to ensure that our error logic won't stop on the first attribute but will instead store errors after attempting to parse all attributes:

```rust
:::-> file.write cache_diff/tests/fails/multiple_unknown.rs
use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct CustomDiffFn {
    #[cache_diff(unknown)]
    #[cache_diff(unknown = "value")]
    #[cache_diff(unknown = function)]
    name: String,
}

fn main() {}
```

And assert the output of that failure case:

```
:::>> $ cat cache_diff/tests/fails/multiple_unknown.stderr
```

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
:::>> $ cat cache_diff_derive/src/shared.rs
:::>> $ cat cache_diff_derive/src/parse_field.rs
:::>> $ cat cache_diff_derive/src/parse_container.rs
```
</details>

We need to adjust our container code to add a container attribute, and then our final `lib.rs` code needs to use all this tasty info we just added.

## Start at the very end: Parse the attributes

> Skip this if you want to be really confused later.

We talked about field attributes earlier. That was the `#[serde(rename = "ruby_version")]` that annotated `version: String`. We know we want a `display = <function>` and `ignore` attributes.

In proc macro land a function can be held as a `syn::Path` this could be a function like `display` if we have one in scope or a fully qualified path like `std::path::PathBuf::display`. Note that the function doesn't have parens here because we're not calling it yet. Syn has a handful of types that you'll interact with. One worth calling out is `syn::Ident` which is short for "identity." I think of the "identity" of a code as the literal that describes it. For example the identity of `struct Metadata` is `Metadata`. You'll get a feel for various types as you use them.

The high level strategy for how we'll move forward is by defining data structures that represent the state we want to hold. We need something to represent a single attribute like `display = <function>` then we need to take those attributes and combine them. We combine that attribute information with other data from the source code (like field names and struct identity) and then FINALLY we use that final representation to generate our trait programatically. It sounds like a lot, but this strategy lets us leverage Rust types as much as possible to help prevent error. It will make more sense once you see it.

If that sounds like a lot, it is. To help us out, we'll use a crate called `darling`. It's not required, but it helps simplify some common tasks. We want the data from a single set of field attributes to look like this:

```rust
:::>> file.write cache_diff_derive/src/field_data.rs
use darling::FromAttributes;

#[derive(Debug, FromAttributes)]
#[darling(attributes(cache_diff))]
struct ParsedAttributes {
    #[darling(default)]
    display: Option<syn::Path>,
    #[darling(default)]
    ignore: bool,
}
```

And let the project know about the new file:

```
:::>> file.append cache_diff_derive/src/lib.rs#1
mod field_data;
```

Let's break the code we just added down. We created a struct `ParsedAttributes` that holds an optional `syn::Path` for `display` and an `ignore` field that can be true or false. We then inform it that it's supposed to hold attributes so we derive debug and `#[derive(Debug, FromAttributes)]`. Then we tell it the namespace of our attributes: `#[darling(attributes(cache_diff))]`.

Since neither attribute is required, we mark both as having a default via `#darling(default)`. With this data structure we can now parse attributes. Let's add some tests to prove it:


```rust
:::>> file.append cache_diff_derive/src/field_data.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_attribute_parsing() {
        let input: Vec<syn::Attribute> =
            syn::parse_quote! { #[cache_diff(display = std::path::PathBuf::display)] };
        let result = ParsedAttributes::from_attributes(&input).unwrap();
        assert!(matches!(
            result,
            ParsedAttributes {
                display: Some(_),
                ignore: false
            },
        ));
    }
}
```

This test uses `syn::parse_quote` to parse `#[cache_diff(display = std::path::PathBuf::display)]` into a `Vec<syn::Attribute>`, we then pass it into the derived function `from_attributes` given to us via darling. Finally we assert that we got some display and ignore defaulted to false.

Now let's test the `ignore` flag:

```rust
    #[test]
    fn test_ignore_attribute_parsing() {
        let input: Vec<syn::Attribute> =
            syn::parse_quote! { #[cache_diff(ignore)] };
        let result = ParsedAttributes::from_attributes(&input).unwrap();
        assert!(matches!(
            result,
            ParsedAttributes {
                display: None,
                ignore: true
            },
        ));
    }
}
```

This code we just added does pretty much the same thing as before but takes in `#[cache_diff(ignore)]` and returns a display of `None` and the ignore flag is true.

The whole file should look like this:

```
:::>> $ cat cache_diff_derive/src/field_data.rs
```

Verify our tests work:

```
:::>- $ cargo test
```

> Protip: I use either `bacon` or `cargo watch` to automatically run my tests on project save. It's really handy. I recommend it.

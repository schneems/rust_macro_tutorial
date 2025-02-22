## Define a Field

> Skip this if you don't want your code to compile

You may recall that a field in our context refers to a name and type within a struct. We need a way to model this in our code so we can add onto it later:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_field.rs", code: <<-CODE)
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
}
CODE
%>
```

The interesting bits here is that the `ident` field holds a `syn::Ident` which is shorthand for an "identifier" of Rust code, we'll use this when we want to compare one field value to another so `old.version != new.version` would become `old.#ident != new.#ident`. Then we store the name of the field we want to show when a difference is detected. We want it to look nice, so instead of showing a string like `"ruby_version"` we'll convert it to `"ruby version"` (with a space instead of an underscore). This isn't strictly required at this point, but we're laying a foundation to build on.

To use this code we've got to `mod` it from the `lib.rs`

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/lib.rs", use: "mod parse_field;\n") %>
```

Now that we've got somewhere to put data, we need some logic:

```rust
:::>> print.erb
<% import = ["use crate::{MACRO_NAME, NAMESPACE};"] %>
<% import << "use syn::spanned::Spanned;\n" %>
<% code = <<-CODE
impl ParseField {
    pub(crate) fn from_field(field: &syn::Field) -> Result<Self, syn::Error> {
        let ident = field.ident.clone().ok_or_else(|| {
            syn::Error::new(
                field.span(),
                format!("{MACRO_NAME} can only be used on structs with named fields"),
            )
        })?;

        let name = ident.to_string().replace("_", " ");

        Ok(ParseField {
            ident,
            name,
        })
    }
}
CODE
%>
<%=
append(filename: "cache_diff_derive/src/parse_field.rs", use: import, code: code)
%>
```

The function we added takes in a `syn::Field`, a parsing abstraction over a struct's fields and returns itself of a `syn::Error`:

```rust
:::-> $ grep -A1000 'pub(crate) fn from_field' cache_diff_derive/src/parse_field.rs | awk '/{/ {print; exit} {print}'
```

Syn provides a number of these abstractions out of the box, you can find a list at TODO.

This next bit of code pulls out the identity of the field if there is one, or returns `syn::Error` one isn't provided:

```rust
:::-> $ grep -A1000 'let ident' cache_diff_derive/src/parse_field.rs | awk '/\;/ {print; exit} {print}'
```

As the error message suggests, this might be `None` if someone tried to use our derive macro on something that has a field without an ident like a tuple struct (i.e. `struct Metadata(String)`). The first argument of the error takes in a `Span`. This is a common parsing abstraction, it represents a sequence of characters in our input. The `syn` crate uses this information to make nice error messages with references to our code that Rust developers have come to expect. To get the span from the field input we have to import the `Spanned` trait.

From there we gather the string representation of our identity, and replace underscores with spaces:

```rust
:::-> $ grep -A1000 'let name' cache_diff_derive/src/parse_field.rs | awk '/\;/ {print; exit} {print}'
```

But don't take my word for it, let's see the code in action:

```rust
:::>> print.erb
<% import = ["    use super::*;"] %>
<% import << "    use syn::parse::Parse;\n" %>
<% append(filename: "cache_diff_derive/src/parse_field.rs", test_use: import, test_code: <<-CODE)
    #[test]
    fn test_parse_field_plain() {
        let field: syn::Field = syn::parse_quote! {
            ruby_version: String
        };

        let parsed = ParseField::from_field(&field).unwrap();
        assert_eq!("ruby version".to_string(), parsed.name);
    }
CODE
%>
```

This code uses `syn::parse_quote!` macro to generate a `syn::Field` that we can use to pass to the associated function we just defined. We have to annotate the type in the test or syn won't know what data structure we're trying to represent in our code. From there it asserts our naming logic works as expected.

Happy paths are nice and all, but what about that error from earlier?

```rust
:::>> print.erb
<% append(filename: "cache_diff_derive/src/parse_field.rs", test_code: <<-CODE)
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
CODE
%>
```

> Protip: To assert the full output, you could use the [trybuild crate]().

This is our code so far:

```rust
:::-> $ cat cache_diff_derive/src/parse_field.rs
```

Run tests to make sure everything works as expected:

```
:::>- $ cargo test
```

Now that we have a representation for a field, let's model our container.

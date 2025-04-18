<span id="chapter_03" />

## 03: Create a ParseField to Derive with

Many aspiring drivers learn in an empty parking lot. We'll start Deriving for real with an empty field.

A field in our context refers to a name and type within a struct. For example:

```rust
struct Metadata {
    version: String
}
```

This struct has one field named `version` with a value of `String` type. We need a way to model this concept in our code and add it later. Create this file and add this code now:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_field.rs", code: <<-CODE)
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
}
CODE
%>
```

The `ident` field on `ParseField` holds a `syn::Ident`, which is shorthand for an "identifier" of Rust code. We'll use this when we want to compare one field value to another, so `old.version != new.version` would become `old.#ident != new.#ident`. The [syn crate](https://crates.io/crates/syn) ships with many [pre-defined data structures that represent various Rust code](https://docs.rs/syn/2.0.99/syn/#structs) that we can easily parse a `TokenStream` into.

Then, we store the `name` of the field we want to show when a difference is detected. We want it to look nice, so instead of showing a string like `"ruby_version"` we'll convert it to `"ruby version"` (with a space instead of an underscore). This conversion isn't strictly required, but it makes the output prettier, and decoupling the ident from its display representation will be used later.

We must `mod` it from the `lib.rs` to use this code. Do that now:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/lib.rs", mod: "mod parse_field;") %>
```

Now that we've got somewhere to put data, we need some logic to build it. Add this code:

```rust
:::>> print.erb
<% import = ["use crate::MACRO_NAME;"] %>
<% import << "use syn::spanned::Spanned;" %>
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

The function we added takes in a `syn::Field`, a parsing abstraction over a struct's fields, and returns itself of a `syn::Error`:

```rust
:::-> $ grep -A1000 'pub(crate) fn from_field' cache_diff_derive/src/parse_field.rs | awk '/{/ {print; exit} {print}'
```

This next bit of code pulls out the identity of the field if there is one or returns `syn::Error`:

```rust
:::-> $ grep -A1000 'let ident' cache_diff_derive/src/parse_field.rs | awk '/\;/ {print; exit} {print}'
```

As the error message suggests, this might be `None` if someone tried to use the derive macro on something with a field without an ident like a tuple struct (i.e. `struct Metadata(String)`). The first argument of the error takes in a `syn::Span`. A span is a common parsing abstraction not unique to syn or Rust, it represents a sequence of characters in our input. The `syn` crate uses this information to add underlines and arrows that Rust developers have come to expect. To use the span from the field input, we have to import the `syn::spanned::Spanned` trait.

You can also panic in a macro, but part of being a great Deriver is about going above and beyond to signal intent and help others.

From there, we gather the string representation of our identity and replace underscores with spaces:

```rust
:::-> $ grep -A1000 'let name' cache_diff_derive/src/parse_field.rs | awk '/\;/ {print; exit} {print}'
```

That's how the code works, but don't take my word for it. Let's see the code in action. Add a test:

```rust
:::>> print.erb
<% import = ["    use super::*;"] %>
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

This test code uses the `syn::parse_quote!` macro to generate a `syn::Field` that we can pass to the associated function we just defined. We have to annotate the type in the test, or `syn` won't know what data structure we're trying to represent in our code. From there, it asserts that our naming logic works as expected.

Happy paths are nice, but what about that error from earlier? Add a test now:

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

This is our code so far:

```rust
:::-> $ cat cache_diff_derive/src/parse_field.rs
```

Run tests to make sure everything works as expected:

```
:::>- $ cargo test
```

Now that we have a field for Deriving let's model our container.

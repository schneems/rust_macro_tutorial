<span id="chapter_04" />

## 04: Create a ParseContainer to Derive with

A container, in our context, is a struct. For example:

```rust
struct Metadata {
    version: String
}
```

The container in this code is `Metadata` as it contains our fields. In proc-macro land, a container can also be an enum. We want a way to model a container that holds zero or more named fields. Create this file and add this code:

```rust
:::>> print.erb
<%
import = ["use crate::parse_field::ParseField;"]
%>

<%= append(filename: "cache_diff_derive/src/parse_container.rs", use: import, code: <<-CODE)
/// Container (i.e. struct Metadata { ... }) and its parsed attributes
/// i.e. `#[cache_diff( ... )]`
#[derive(Debug)]
pub(crate) struct ParseContainer {
    /// The proc-macro identifier for a container i.e. `struct Metadata { }` would be a programmatic
    /// reference to `Metadata` that can be used along with `quote!` to produce code.
    pub(crate) ident: syn::Ident,
    /// Info about generics, lifetimes and where clauses i.e. `struct Metadata<T> { name: T }`
    pub(crate) generics: syn::Generics,
    /// Fields (i.e. `name: String`) and their associated attributes i.e. `#[cache_diff(...)]`
    pub(crate) fields: Vec<ParseField>,
}
CODE

%>
```

Like before, we're holding a reference to `syn::Ident`, which holds the identity of the struct (i.e., `Metadata`). Then, we're also capturing the `syn::Generics`, which holds information needed to support things like structs with generics or where clauses. Then, instead of storing a `syn` data type for fields, we're holding a `Vec` of the `ParseField` struct we defined previously.

Don't forget to let our project know about the new file by adding a `mod` declaration. Add it now:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/lib.rs", mod: "mod parse_container;") %>
```

Now that we've got a place to hold the data let's build it from the input AST. Add this code:

```rust
:::>> print.erb
<%= append(filename: "cache_diff_derive/src/parse_container.rs", use: "use crate::MACRO_NAME;", code: <<-CODE)
impl ParseContainer {
    pub(crate) fn from_derive_input(input: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let ident = input.ident.clone();
        let generics = input.generics.clone();
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

        Ok(ParseContainer { ident, generics, fields })
    }
}
CODE
%>
```

What does this code do? The function takes in a `syn::DeriveInput` and returns itself or a `syn::Error` (just like `ParseField` did!):

```rust
:::-> $ grep -A1000 'pub(crate) fn from_derive_input' cache_diff_derive/src/parse_container.rs | awk '/{/ {print; exit} {print}'
```

While `ParseField` took in a `syn::Field`, there's no pre-defined "container" type from syn; instead, `syn::DeriveInput` is anything that could be passed to a derive macro. You can mentally substitute the word "container" whenever you see `DeriveInput` because derive macros can only be applied to containers. If you need a mental image, think of a big rig (truck) driving to your town containing lots of goodies. Containers are derivable.

> Derive input == container == (struct or enum or union)

All containers for our macro are named, so we can pull an identity directly (without needing to raise an error as we did with fields):

```rust
:::-> $ grep -A1000 'let ident' cache_diff_derive/src/parse_container.rs | awk '/\;/ {print; exit} {print}'
```

This next bit is tricky. We will break it down:

```rust
:::-> $ grep -A1000 'let fields =' cache_diff_derive/src/parse_container.rs | awk '/\;/ {print; exit} {print}'
```

Because a `DeriveInput` (A.K.A. "container") can take different shapes (struct, enum, or union), we can use a match statement to extract the information we need from named fields.

The return value from `syn::Data` here is a `&syn::Punctuated<syn::Field, syn::Token::Comma>`, which is a fancy way of saying that it's a `syn::Field` that is separated by commas. We can iterate over that type to yield `&syn::Field`, which is what our `ParseField::from_field` function takes in:

```rust
:::-> $ grep -A1000 'into_iter()' cache_diff_derive/src/parse_container.rs | awk '/\;/ {print; exit} {print}'
```

If that parsing is successful, then we'll have our data! But not so fast **speed racer**. We can't pass GO until we write some tests. Add this test code now:

```rust
:::>> print.erb
<% import = "    use super::*;" %>
<%= append(filename: "cache_diff_derive/src/parse_container.rs", test_use: import, test_code: <<CODE)
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
CODE
%>
```

Verify it works:

```
:::>- $ cargo test
```

At this point, we've got a custom representation for our fields and the container (that holds the fields). We'll use this to generate a simple version of our trait before extending our simple data structures to hold attribute information.

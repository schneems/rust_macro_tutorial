<span id="chapter_05"/>

## 05: Implement the basic Derive macro

We will now use these container and field structs that we created to implement our base logic. Import the structs we created:

```rust
:::>> print.erb
<% import = ["use parse_container::ParseContainer;"] %>
<% import << "use parse_field::ParseField;" %>
<%= append(filename: "cache_diff_derive/src/lib.rs", use: import) %>
```

Replace our prior cache logic:

```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/lib.rs", match: /fn create_cache_diff/,  code: <<-CODE)
fn create_cache_diff(item: proc_macro2::TokenStream)
    -> syn::Result<proc_macro2::TokenStream> {
    let derive_input: syn::DeriveInput = syn::parse2(item)?;
    let container = ParseContainer::from_derive_input(&derive_input)?;
    let ident = &container.ident;
    let generics = &container.generics;

    let mut comparisons = Vec::new();
    for field in container.fields.iter() {
        let ParseField {
            ident,
            name,
            ..
        } = field;

        comparisons.push(quote::quote! {
            if self.#ident != old.#ident {
                differences.push(
                    format!("{name} ({old} to {new})",
                        name = #name,
                        old = &old.#ident,
                        new = &self.#ident,
                    )
                );
            }
        });
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    Ok(quote::quote! {
        impl #impl_generics ::cache_diff::CacheDiff for #ident #type_generics #where_clause {
            fn diff(&self, old: &Self) -> ::std::vec::Vec<String> {
                let mut differences = ::std::vec::Vec::new();
                #(#comparisons)*
                differences
            }
        }
    })
}
CODE
%>
```

That's a lot of code, let's break it down. The first thing we do is generate a `syn::DeriveInput` from the token stream:

```rust
:::-> $ grep 'let derive_input' cache_diff_derive/src/lib.rs
```

The function `syn::parse2` is especially designed to turn a `proc_macro2::TokenStream` into a syn struct such as `syn::DeriveInput`. Next we build our container structure from this parsed input:

```rust
:::-> $ grep 'let container' cache_diff_derive/src/lib.rs
```

We will need the identity of the container (i.e. `Metadata`) for code generation later with the `quote::quote!` macro:

```rust
:::-> $ grep 'let ident' cache_diff_derive/src/lib.rs
:::>> print.text "    // ..."
:::-> $ grep -A1000 'Ok(quote::quote! {' cache_diff_derive/src/lib.rs | awk '/})/ {print; exit} {print}'
```

In the above code, variables can be substituted in order to generate code by starting with a pound (`#`). For example, `#ident` will be replaced with ident from the `let let` variable. The `#(#comparisons)*` code expands the `let comparisons` variable which contains a `Vec<proc_macro2::TokenStream>` which is generated via the `quote::quote!` macro (which we'll look into in a minute). You can read more about [this syntax in the quote docs](https://docs.rs/quote/1.0.38/quote/macro.quote.html#interpolation). From the quote docs:

> Repetition is done using #(...)* or #(...),* again similar to macro_rules!. This iterates through the elements of any variable interpolated within the repetition and inserts a copy of the repetition body for each one. The variables in an interpolation may be a Vec, slice, BTreeSet, or any Iterator.

We skipped ahead of how we generated those comparisons. Let's go back and look at it now:

```rust
:::-> $ grep -A1000 'let mut comparisons' cache_diff_derive/src/lib.rs |  awk '/^    })/ {print; exit} {print}'
```

In this code we're looping through all of the fields and pulling out the identifier (i.e. `version` for `version: String`), as well as the un-underscored name. Like we saw with the struct identifier, we will use the `quote::quote!` macro and the inner variables `ident` and `name` to check if the current value does not equal the old value, and if that happens then format that information and add it to the vec.

Unfortunately we cannot test the derive macro invocation in the same crate, because the macro must be compiled first. However, we can test it in our original crate.

We can use Rust's doctests to validate the happy path. At the top of `cache_diff/src/lib.rs` add module docs with a doctest that uses our derive macro:

```rust
:::>> print.erb
<%
module_docs = <<~EOF
//! Cache Diff (derive)
//!
//! Generate the difference between two structs for the purposes of cache invalidation.
//!
//! Example:
//!
//! #{BACKTICKS}
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff, Debug)]
//! struct Metadata {
//!     ruby_version: String,
//!     architecture: String,
//! }
//!
//! let diff = Metadata {ruby_version: "3.4.2".to_string(), architecture: "arm64".to_string()}
//!     .diff(&Metadata {ruby_version: "3.3.1".to_string(), architecture: "amd64".to_string()});
//!
//! assert_eq!(
//!     vec!["ruby version (3.3.1 to 3.4.2)".to_string(), "architecture (amd64 to arm64)".to_string()],
//!     diff
//! );
//! #{BACKTICKS}
EOF
%>
<%= append(filename: "cache_diff/src/lib.rs", module_docs: module_docs) %>
```

This derive macro test is asserting the same behavior we implmemented manually before:

```rust
:::-> $ grep -A1000 'impl CacheDiff for Metadata' cache_diff/src/lib.rs | awk '/new.diff(&old)/ {print; exit} {print}'
```

Now verify it all works:

```
:::>- $ cargo test
```

Great! If your project is failing or if the tests you added didn't run, here's the full project for reference:

<details>
  <summary>Full project</summary>

```
:::>> $ exa --tree --git-ignore .
:::>> $ cat Cargo.toml
:::>> $ cat cache_diff/Cargo.toml
:::>> $ cat cache_diff_derive/Cargo.toml
:::>> $ cat cache_diff/src/lib.rs
:::>> $ cat cache_diff_derive/src/lib.rs
:::-- $ touch cache_diff_derive/src/shared.rs
:::>> $ cat cache_diff_derive/src/shared.rs
:::>> $ cat cache_diff_derive/src/parse_field.rs
:::>> $ cat cache_diff_derive/src/parse_container.rs
```
</details>

Great! Our macro needs to be able to handle any possible valid Rust code input. You may have noticed we needed to explicitly extract information about generics and use that to generate our trait:

```rust
:::-> $ grep -A1000 'split_for_impl()' cache_diff_derive/src/lib.rs |  awk '/#type_generics/ {print; exit} {print}'
```

To verify that our code works with generics you can add a test for that behavior:

```rust
//!
//! #{BACKTICKS}
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff, Debug)]
//! struct Metadata<T> {
//!     ruby_version: String,
//!     architecture: T,
//! }
//!
//! let diff = Metadata<String> {ruby_version: "3.4.2".to_string(), architecture: "arm64".to_string()}
//!     .diff(&Metadata<String> {ruby_version: "3.3.1".to_string(), architecture: "amd64".to_string()});
//!
//! assert_eq!(
//!     vec!["ruby version (3.3.1 to 3.4.2)".to_string(), "architecture (amd64 to arm64)".to_string()],
//!     diff
//! );
//! #{BACKTICKS}
EOF
%>
<%= append(filename: "cache_diff/src/lib.rs", module_docs: module_docs) %>
```

Congrats! You just wrote a derive macro! But we're not done yet. Now, that we have the base functionality in place, let's look a little bit a derive macro attributes so we can make our trait Derive easy to customize.


<span id="chapter_09" />

## 09: Implement the All-Wheel Derive Macro (customizable with attributes)

With the parsing logic contained within `ParseContainer` and `ParseField`, we can focus on implementing the core logic of our macro. Replace this code:

```rust
:::-> print.erb
<%=
replace(filename: "cache_diff_derive/src/lib.rs", match: /fn create_cache_diff/, code: <<-CODE )
fn create_cache_diff(item: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let ParseContainer {
        ident,
        generics,
        custom,
        fields,
    } = ParseContainer::from_derive_input(&syn::parse2(item)?)?;

    let custom_diff = if let Some(ref custom_fn) = custom {
        quote::quote! {
            let custom_diff = #custom_fn(old, self);
            for diff in &custom_diff {
                differences.push(diff.to_string())
            }
        }
    } else {
        quote::quote! {}
    };

    let mut comparisons = Vec::new();
    for field in fields.iter() {
        let ParseField {
            ident,
            name,
            ignore,
            display,
        } = field;

        if ignore.is_none() {
            comparisons.push(quote::quote! {
                if self.#ident != old.#ident {
                    differences.push(
                        format!("{name} ({old} to {new})",
                            name = #name,
                            old = #display(&old.#ident),
                            new = #display(&self.#ident)
                        )
                    );
                }
            });
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    Ok(quote::quote! {
        impl #impl_generics ::cache_diff::CacheDiff for #ident #type_generics #where_clause {
            fn diff(&self, old: &Self) -> ::std::vec::Vec<String> {
                let mut differences = ::std::vec::Vec::new();
                #custom_diff
                #(#comparisons)*
                differences
            }
        }
    })
}
CODE
%>
```

One thing to call out here is that I'm using `::std::vec::Vec<String>`. The beginning `::` is because the environment where the generated code will live is not hygienic." The [rust reference](https://doc.rust-lang.org/reference/procedural-macros.html#procedural-macro-hygiene) says more. But basically, if you used `Vec`, then the calling code could change the behavior of your output by accident if they `use some_other_thing as Vec`. So, to avoid ambiguity, we use full paths and start them with `::` (otherwise, someone could `use other_thing as std`).

The logic inside of the function is similar to what we saw before. Pull out values from the parsed token stream using `syn`. Use those values to generate rust code with `quote`. Like before, we will write doctests that use our features like a user would. Beyond convincing you that the code we wrote works, this documentation will be easy to find for anyone using the macro.

I like to make one example per (major) feature. And use something close to the real-world reason why I added the feature. In my [book on contributing to open source](http://howtoopensource.dev/), there is a  documentation chapter where we document code someone else wrote, I stressed that documentation should help answer the question "why does this (feature) exist." Great documentation doesn't just say why the code exists. It shows it.

Add docs for `ignore` now:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", module_docs: <<-CODE)
//! ## Ignore attributes
//!
//! If the struct contains fields that should not be included in the diff comparison, you can ignore them:
//!
//! #{BACKTICKS}rust
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff)]
//! struct Metadata {
//!     version: String,
//!
//!     #[cache_diff(ignore)]
//!     changed_by: String
//! }
//! let now = Metadata { version: "3.4.0".to_string(), changed_by: "Alice".to_string() };
//! let diff = now.diff(&Metadata { version: now.version.clone(), changed_by: "Bob".to_string() });
//!
//! assert!(diff.is_empty());
//! #{BACKTICKS}
CODE
%>
```

Add rename docs:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", module_docs: <<-CODE)
//! ## Rename attributes
//!
//! If your field name is not descriptive enough, you can rename it:
//!
//! #{BACKTICKS}rust
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff)]
//! struct Metadata {
//!     #[cache_diff(rename="Ruby version")]
//!     version: String,
//! }
//! let now = Metadata { version: "3.4.0".to_string() };
//! let diff = now.diff(&Metadata { version: "3.3.0".to_string() });
//!
//! assert_eq!("Ruby version (3.3.0 to 3.4.0)", diff.join(" "));
//! #{BACKTICKS}
//!
CODE
%>
```

Add display docs:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", module_docs: <<-CODE)
//! ## Handle structs missing display
//!
//! Not all structs implement the [`Display`](std::fmt::Display) trait, for example [`std::path::PathBuf`](std::path::PathBuf) requires that you call `display()` on it.
//!
//! The `#[derive(CacheDiff)]` macro will automatically handle the following conversions for you:
//!
//! - `std::path::PathBuf` (via [`std::path::Path::display`](std::path::Path::display))
//!
//! However, if you have a custom struct that does not implement [`Display`](std::fmt::Display), you can specify a function to call instead:
//!
//! #{BACKTICKS}rust
//! use cache_diff::CacheDiff;
//!
//! #[derive(CacheDiff)]
//! struct Metadata {
//!     #[cache_diff(display = my_function)]
//!     version: NoDisplay,
//! }
//!
//! #[derive(PartialEq)]
//! struct NoDisplay(String);
//! fn my_function(s: &NoDisplay) -> String {
//!     format!("custom {}", s.0)
//! }
//!
//! let now = Metadata { version: NoDisplay("3.4.0".to_string())};
//! let diff = now.diff(&Metadata { version: NoDisplay("3.3.0".to_string())});
//!
//! assert_eq!("version (custom 3.3.0 to custom 3.4.0)", diff.join(" "));
//! #{BACKTICKS}
//!
CODE
%>
```

Add custom function docs:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", module_docs: <<-CODE)
//! ## Customize one or more field differences
//!
//! You can provide a custom implementation for a diffing a subset of fields without having to roll your own implementation.
//!
//! ### Custom logic for one field example
//!
//! Here's an example where someone wants to bust the cache after N cache calls. Everything else other than `cache_usage_count` can be derived. If you want to keep the existing derived difference checks, but add on a custom one you can do it like this:
//!
//! #{BACKTICKS}rust
//! use cache_diff::CacheDiff;
//! const MAX: f32 = 200.0;
//!
//! #[derive(Debug, CacheDiff)]
//! #[cache_diff(custom = diff_cache_usage_count)]
//! pub(crate) struct Metadata {
//!     #[cache_diff(ignore = "custom")]
//!     cache_usage_count: f32,
//!
//!     binary_version: String,
//!     target_arch: String,
//!     os_distribution: String,
//!     os_version: String,
//! }
//!
//! fn diff_cache_usage_count(_old: &Metadata, now: &Metadata) -> Vec<String> {
//!     let Metadata {
//!         cache_usage_count,
//!         binary_version: _,
//!         target_arch: _,
//!         os_distribution: _,
//!         os_version: _,
//!     } = now;
//!
//!     if cache_usage_count > &MAX {
//!         vec![format!("Cache count ({}) exceeded limit {MAX}", cache_usage_count)]
//!     } else {
//!         Vec::new()
//!     }
//! }
//! #{BACKTICKS}
//!
//! In this example, four fields are derived automatically, saving us time, while one field is custom
//! using the `#[cache_diff(custom = diff_cache_usage_count)]` attribute on the struct. This tells
//! [CacheDiff] to call this function and pass in the old and current values. It expects a vector
//! with some strings if there is a difference and an empty vector if there are none.
//!
//! Don't forget to `#[cache_diff(ignore = "custom")]` any fields you're implementing yourself. You can also use this feature to
//! combine several fields into a single diff output, for example, using the previous struct, if
//! you only wanted to have one output for a combined `os_distribution` and `os_version` in one output
//! like "OS (ubuntu-22 to ubuntu-24)". Alternatively, you can use <https://github.com/schneems/magic_migrate> to
//! re-arrange your struct to only have one field with a custom display.
//!
CODE
%>
```

And make sure it works as expected:

```
:::>> $ cargo clippy
:::>> $ cargo test
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


If your code compiles, congratulations, you just earned your Derive-ing license!

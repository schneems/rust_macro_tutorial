<span id="chapter_03" />

## Create an empty Proc Macro

To write a proc macro we will need some crates. Add them now:

```toml
:::>> file.append cache_diff_derive/Cargo.toml
# Turn Rust code into Tokens
quote = "1.0.37"
# Parse tokens into Rust code
syn = { version = "2.0.83", features = ["extra-traits"] }
proc-macro2 = "1.0.89"
```

I'll talk about what these do as we use them. We also need to tell rust that this library is a proc macro.

```toml
:::>> file.append cache_diff_derive/Cargo.toml
[lib]
proc-macro = true
```

Now we need to add an entrypoint and define some constants we'll use in a bit. Add this code:

```rust
:::>> print.erb
<% import = "use proc_macro::TokenStream;" %>
<% code_blocks = [] %>
<% code_blocks << <<-EOF
pub(crate) const NAMESPACE: &str = "cache_diff";
pub(crate) const MACRO_NAME: &str = "CacheDiff";

#[proc_macro_derive(CacheDiff, attributes(cache_diff))]
pub fn cache_diff(item: TokenStream)
    -> TokenStream {
    create_cache_diff(item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
EOF
%>
<% code_blocks << <<-EOF
fn create_cache_diff(item: proc_macro2::TokenStream)
    -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote::quote! { })
}
EOF
%>
<%= append(filename: "cache_diff_derive/src/lib.rs", use: import, code: code_blocks)
%>
```

What does this code do? We just created a function `cache_diff` that is annotated with a proc macro (how meta):

```rust
:::-> $ grep 'proc_macro_derive' cache_diff_derive/src/lib.rs
```

This line says that we provide a derive macro named `CacheDiff` and that it should accept some attributes that start with `#[cache_diff()]`. Originally I thought that meant it would only show me the attributes I was interested in (i.e. prefixed with `cache_diff`), but it doesn't, the proc macro sees all attributes in the source code. If the struct has different attributes from different crates (such as `serde`) then it can see them, we need to manually filter attributes later so we only see the ones we care about.

The next bit defines a function `cache_diff` that will receive a `proc_macro::TokenStream` containing information about the code we're annotating:

```rust
:::-> $ grep -A1000 'pub fn cache_diff' cache_diff_derive/src/lib.rs | awk '/^}/ {print; exit} {print}'
```

It calls a function `create_cache_diff` which returns a `syn::Result<proc_macro2::TokenStream>>`. That's either a parse error or a stream of tokens. In the event of a problem, we want to map it into a pretty compile error with source code highlighting where the issue happend with underlines and arrows and all that nice output that Rust users know and love. Inside of the `create_cache_diff` function I added a call to the `quote::quote!` macro:

```rust
:::-> $ grep -A1000 'fn create_cache_diff' cache_diff_derive/src/lib.rs | awk '/^}/ {print; exit} {print}'
```

This macro takes converts text into rust code, we can also pass in variables, but for now we just want the code to compile. Verify your code builds:

```term
:::>> print.text $ cargo build
:::-- $ cargo build --offline
```

Congrats! You just wrote your first proc macro! To use it we'll need to expose it through our non-derive crate that also carries the trait definition. First declare a dependency on our derive crate:

```toml
:::>> file.append cache_diff/Cargo.toml
cache_diff_derive = { version = "0.1.0" , optional = true, path = "../cache_diff_derive" }

[features]
derive = ["dep:cache_diff_derive"]
default = ["derive"]
```

This declares a dependency on our derive macro. It's optional because proc macros are "heavier" than a regular dependency in that they have to compile and execute before your code can compile and execute. Someone might want to pull in only the trait to use it as an interface or to manually implement it for a struct (meaning they don't need our automation). Many popular libraries such as [the clap CLI builder](https://github.com/clap-rs/clap/blob/fdbbf66e77c83688f52b7a206d64102582af40d3/Cargo.toml#L161) gate their proc macros behind a feature. While many users will use the derive features, those who don't won't have to pay the (relatively small in this case, but very real) cost. I'm making the assumption that people want to use it by default, so it's enabled automatically. Users can disable it by specifying `cache_diff = { default-features = false }` in the `Cargo.toml` if they want.

Now we re-export that macro right next to our trait, when the "derive" feature is enabled:

```rust
:::>> print.erb
<%= append(
    filename: "cache_diff/src/lib.rs",
    use: ['#[cfg(feature = "derive")]', "pub use cache_diff_derive::CacheDiff;"]) %>
```

The file should look like this:

```rust
:::-> $ cat cache_diff/src/lib.rs
```

With this skeleton in place, we will define structs to hold data, this will allow us to implement the base behavior and gradually add on custom attributes.

## Proc, meet macro

> Skip this section if: You know how to start a proc macro project and are somehow not aware that you're reading a proc macro tutorial.

To write a proc macro we will need some crates. It's easier to see what they do as we use them.

```toml
:::>> file.append cache_diff_derive/Cargo.toml
# Turn Rust code into Tokens
quote = "1.0.37"
# Parse tokens into Rust code
syn = { version = "2.0.83", features = ["extra-traits"] }
proc-macro2 = "1.0.89"
```

We also need to tell rust that this library is a proc macro.

```toml
:::>> file.append cache_diff_derive/Cargo.toml
[lib]
proc-macro = true
```

Now we need to add an entrypoint:

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

This creates a function `cache_diff` that is annotated with a proc macro (how meta):

```rust
:::-> $ grep 'proc_macro_derive' cache_diff_derive/src/lib.rs
```

This line says that we provide a derive macro named `CacheDiff` and that it should accept some attributes that start with `#[cache_diff()]`. Originally I thought that meant it would only show me the attributes I was interested in, but it doesn't. If the struct has different attributes from different crates (remember `serde`?) then we can see them, we need to manually filter attributes so we only see the ones we care about.

The next bit defines a function `cache_diff` that will receive a `proc_macro::TokenStream` containing information about the code we're annotating:

```rust
:::-> $ grep -A1000 'pub fn cache_diff' cache_diff_derive/src/lib.rs | awk '/^}/ {print; exit} {print}'
```

It calls a function `create_cache_diff` which returns a `syn::Result<proc_macro2::TokenStream>>`. That's effectively either a parse error or a stream of tokens. In the event of an error we want to map it into a pretty compile error with source code highlighting where the problem happend that Rust users know and love. Inside of the `create_cache_diff` function I added a call to the `quote::quote!` macro:

```rust
:::-> $ grep -A1000 'fn create_cache_diff' cache_diff_derive/src/lib.rs | awk '/^}/ {print; exit} {print}'
```

This macro takes converts text into rust code, we can also pass in variables, but for now we just want the code to compile:

```term
:::>- $ cargo build
```

Congrats! You just wrote your first proc macro! To use it we'll need to expose it through our non-derive crate that also carries the trait definition. First declare a dependency on our derive crate:

```toml
:::>> file.append cache_diff/Cargo.toml
cache_diff_derive = { version = "0.1.0" , optional = true, path = "../cache_diff_derive" }

[features]
derive = ["dep:cache_diff_derive"]
default = ["derive"]
```

This declares a dependency on our derive macro, it's optional because...why not. Some people might want to rely on the trait and manually implement it without the overhead of pulling in the derive macro. But I'm making the assumption that people want to use it by default.

Now we re-export that macro right next to our trait:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", use: "pub use cache_diff_derive::CacheDiff;") %>
```

The file should look like this:

```rust
:::-> $ cat cache_diff/src/lib.rs
```

With this skeleton in place, we will define structs to hold data, this will allow us to implement the base behavior and gradually add on custom attributes.


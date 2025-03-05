<span id="chapter_07" />

## Understanding attributes for Derive customization

> [Skip](#chapter_08) this if: You are extremely familiar with derive macros and their associated terminology and interfaces.

A derive macro lets us implement traits on data structures with sensible defaults, but what happens when we want something more custom? For that we'll use attributes.

The [`serde`](https://serde.rs/) crate is a (ser)ialiazation/(de)serialization library that ships with derive macros and is customizable via attributes. Here's an example using [v1.0.218](https://docs.rs/serde/1.0.218/serde/index.html):

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Metadata {
    version: String,
    architecture: String,
}
```

And with attributes:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    #[serde(rename = "ruby_version")]
    version: String,
    architecture: String,
}
```

In this code `#[serde(rename = "ruby_version")]` is an attribute on the field `version: String`. This [rename attribute](https://serde.rs/field-attrs.html) over-writes the default field name when serializing. The `#[serde(deny_unknown_fields)]` at the top is an attribute on the container, `Metadata`. This attribute of serde [errors if you try to deserialize a field it's not expecting](https://serde.rs/container-attrs.html). We can use this same attribute concept to customize the behavior of our `CacheDiff` trait generation.

It's convention to name your attribute a lowercased name of your trait, so we will use the `#[cache_diff(...)]` attribute namespace. Technically the inside of the attribute we could use any format or DSL, but it's usually a good idea to mimic existing interfaces that people are already comfortable with. Most attributes take `<key> = <value>` and `<stand-alone-key>` formats. So that 's what we'll use.

To recap:

- A container attribute operates on a data structure like `struct Metadata`.
- A field attribute operates on an individual field of that data structure like `version: String`.

There's another type of attribute called "variant attributes" which apply to enum variants, but we won't be using them in this tutorial.

### Explore the interface space by hand

We could jump write into writing a derive macro, but I want to spend a moment to consider how we want our final project to look like from our user's perspective. In the real world, I prototyped the `CacheDiff` trait, and implemented it manually for several [layers](https://buildpacks.io/docs/for-buildpack-authors/concepts/layer/) to understand the problem space.

### Explore: Don't ignore the signs

One of my structs had some data that I wasn't using as a cache key for invalidation, it was recording the number of times the cache had been written to, when it changed I didn't want that to invalidate the cache. This informed me that I should have a way of skipping or ignoring fields that shouldn't be considered cache keys.

### Explore: Display play

Another struct had a different problem, it contained a [`std::path::PathBuf`](https://doc.rust-lang.org/std/path/struct.PathBuf.html) that should invalidate the cache when it changed. If you've worked with Rust for awhile you might guess the problem, if not I'll spoil the surprise: this type does not directly implement [std::fmt::Display](https://doc.rust-lang.org/std/fmt/trait.Display.html).

What that means is you cannot directly use it in a `format!()` or `println!()` macro or it will error.  If you try to run this code:

```rust
:::-- $ cd ..
:::-- $ cargo new lol
:::-- $ cd lol
:::-> file.write src/main.rs
fn main() {
    println!(
    "Cannot display Path directly {}",
    std::path::PathBuf::from("src/lib.rs")
    );
}
```

Then it will produce this error:

```
:::-> fail.$ cargo test
:::-- $ cd ..
:::-- $ rm -rf lol
:::-- $ cd cache_diff
```

The [std::path::PathBuf::display() function docs](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.display) show how you can call `display()` on it to print or format the value.

What this use case tells me is that I need to provide a way for developers to specify how to `Display` a value even if the type they're using as a cache key doesn't implement it directly.

### Explore: Doc driven design

With those two use cases in mind, we can explore what the interface could look like. We'll use the `Metadata` struct from earlier. We don't have any attribute code yet, but we're going to write some pseduo code of what the interface could look like. This is sometimes called README driven design or Documentation driven design.

Before we saw that an attribute can be a single key like `#[serde(deny_unknown_fields)]`. Since our project is named `CacheDiff` we could implement an `ignore` key like:

```rust
use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct Metadata {
    ruby_version: String,
    architecture: String,

    #[cache_diff(ignore)]
    cache_count: usize
}
```

What about configuring a custom display function? For that we'll need to use a key and value like we saw with `#[serde(rename = "ruby_version")]` but unlike that interface we don't want to configure a static string, we want to give it a path to a dynamic function, thankfully that's possible. We could have the API for that interface look somewhat like this:

```rust
use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct Metadata {
    ruby_version: String,
    architecture: String,

    #[cache_diff(display = std::path::PathBuf::display)]
    binary_location: std::path::PathBuf
}
```

In this code:

```rust
#[cache_diff(display = std::path::PathBuf::display)]
```

The attribute key will be `display` and the path to the function we want to use will be `std::path::PathBuf::display` which is a [function](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.display).

These are sketches of what the code could look like. Here's the IRL README docs that I wrote for these two attributes:

- [Field attribute: `cache_diff(ignore)`](https://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#ignore-attributes)
- [Field attribute: `cache_diff(display = <code path>)`](https://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#handle-structs-missing-display)

In addition to these cutomizations, users also want:

- [The ability to rename  fields. Field attribute: `cache_diff(rename = "<new name>")`](https://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#rename-attributes)
- [Customize cache behavior for some fields without having to manually implement the trait for the rest. Container attribute: `cache_diff(custom = <code path>)`](hhttps://github.com/heroku-buildpacks/cache_diff/blob/fc854c0a1f0e89868bf3d822611dd21229af46f3/cache_diff/README.md#custom-logic-for-one-field-example)

Now that we know what we want the destination to look like, we're ready to modify our code to support attributes!


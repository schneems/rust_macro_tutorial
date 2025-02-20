## Understanding attributes

> Skip this if: You are extremely familiar with derive macros and their associated terminology and interfaces.

A derive macro lets us implement traits on data structures with sensible defaults. A popular library that has a derive macro is [`serde`](https://serde.rs/) a (ser)ialiazation/(de)serialization library. Here's an example:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Metadata {
    version: String,
    architecture: String,
}
```

In addition to being able to define a top level trait that we want to derive, we can also configure it with other values called attributes. It's convention to name your attribute a lowercased name of your trait, so we will use the `#[cache_diff()]` attribute namespace.

Technically we could use any format or DSL inside of the parens, but it's usually a good idea to mimic existing interfaces that people are already comfortable with. Most attributes take `<key> = <value>` and `<stand-alone-key>` formats.  Here's an example of some attributes on serde:

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

There are two attributes here, the first one `#[serde(deny_unknown_fields)]` is a "container" attribute, because it affects the whole `struct Metadata` that it is above. It's called a "container attribute" because it operates on the thing containing the data (struct/enum/etc.). This attribute of serde [errors if you try to deserialize a field it's not expecting](https://serde.rs/container-attrs.html).

The second attribute is `#[serde(rename = "ruby_version")]`. This is a "field attribute" because it applies to the `version: String` field it is above. This [rename attribute](https://serde.rs/field-attrs.html) changes the name of the key used for serialization and deserialization so it will be `ruby_version` instead of `version`.

To recap:

- A container attribute operates on a data structure like `struct Metadata`.
- A field attribute operates on an individual field of that data structure like `version: String`.

There's another type of attribute called "variant attributes" which apply to enum variants, but we won't be using them in this tutorial.

## Explore the interface space by hand

We could jump write into writing a derive macro, but I want to spend a moment to consider how we want our final project to look like from our user's perspective. In the real world, I prototyped that trait, and implemented it manually for several layers to understand the problem space.

## Explore: Don't ignore the signs

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

With those two use cases in mind, we can explore what the interface could look like. We'll use the `Metadata` struct from earlier. We don't have any proc macro code, but we're going to write some pseduo code of what the interface could look like. This is sometimes called README driven design or Documentation driven design.

The simplest case could look like this:

```rust
use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct Metadata {
    version: String,
    architecture: String,
}
```

In this scenario we would create a derive proc macro that mirrors our trait name. The inputs to a derive proc macro is the AST of the Rust code, from this we have access to fields like `version:` and their types like `String`. This example is enough implement the hand-rolled trait trait that we used in our test, but what about our edge cases:

- Ignore a field that shouldn't invalidate the cache
- Display types that don't implement `Display`

Before we saw that an attribute can be a single key like `#[serde(deny_unknown_fields)]`. Since our project is named `CacheDiff` we could implement an `ignore` key like:

```rust
use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct Metadata {
    version: String,
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
    version: String,
    architecture: String,

    #[cache_diff(display = std::path::PathBuf::display)]
    binary_location: std::path::PathBuf
}
```

In this code:

```
#[cache_diff(display = std::path::PathBuf::display)]
```

The attribute key will be `display` and the path to the function we want to use will be `std::path::PathBuf::display` which is a [function](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.display).

These are sketches of what the code could look like. Here's the IRL README docs that I wrote for these two attributes:

- [cache_diff(ignore)](https://github.com/heroku-buildpacks/cache_diff?tab=readme-ov-file#ignore-attributes)
- [cache_diff(display = <code path>)](https://github.com/heroku-buildpacks/cache_diff?tab=readme-ov-file#handle-structs-missing-display)

Now that we know what we want the destination to look like, we're ready to start writing our proc macro!

# Deriving School: Build a Rust proc-macro in one sitting

Welcome to Deriving School. I'll be your Deriving instructor for the day. This tutorial will walk you step-by-step through creating a Rust trait that can be manually implemented, then writing a Derive proc-macro to implement the trait, and then introducing derive macro attributes that can be used to configure the behavior of that code generation. There are many different ways to write a Derive macro; this tutorial heavily emphasizes testing, integration testing, high-quality errors, and enforcing valid state through types and patterns that can be extended for your needs. If that sounds exciting, buckle up, and let's get Deriving!


By the end of this tutorial, we'll have a working `CacheDiff` macro with attributes that function like this:

```rust
#[derive(CacheDiff)]
#[cache_diff(custom = diff_cache_usage_count)]
pub(crate) struct Metadata {
    #[cache_diff(ignore = "custom")]
    cache_usage_count: f32,
    #[cache_diff(rename = "Ruby version")]
    binary_version: String,
    target_arch: String,
    os_distribution: String,
    os_version: String,
}
```

What makes me qualified to be a Deriving instructor? I write Rust code for my day job at Heroku, where I maintain the [Ruby Cloud Native Buildpack](https://github.com/heroku/buildpacks-ruby). If you're unfamiliar with [Cloud Native Buildpacks](https://buildpacks.io/), it's a CNCF project that competes with Dockerfile to generate [OCI images](https://opencontainers.org/). The best introduction to Cloud Native Buildpacks is through a [language-specific tutorial, such as Build a Ruby on Rails application image in 5 minutes, with no Dockerfile required](https://github.com/heroku/buildpacks/blob/main/docs/ruby/README.md). In addition to my day job, I teach elementary school kids how to program. I'm the author of How to Open Source (book) and a Ruby Core Contributor. I'm only a few years into writing professional Rust, but I've got over a decade of experience writing DSLs and working with reflection. Even though I'll be your instructor today, there's still a lot for me to learn; if you have a concrete suggestion on how to make this tutorial better, please get in touch on [Mastodon](https://ruby.social/@schneems).

Before we get started, though, I will need to check your learner's permit. I assume you're comfortable with Rust syntax, have worked through "The Rust book," and can easily manage [rustlings exercises](https://github.com/rust-lang/rustlings) . If that's not the case, I suggest you skim the article today, spend more time on the basics, and return later. If you're ready to start, put on some [good tunes](https://www.youtube.com/watch?v=hEUs9rwNFcs), and let's begin Deriving!

## What is a CacheDiff?

The [CacheDiff trait](https://crates.io/crates/cache_diff) comes from a real-world desire for a standard interface for communicating when a cache must be invalidated and why. In [libcnb.rs](https://crates.io/crates/libcnb), the cache state is represented by a serialized struct, often called metadata. For example, if a target application specifies a version in its metadata, we should clear the old contents when the value changes and communicate why the cache was cleared. For example:

> "Ruby version changed (3.1.3 to 3.4.2)"

As the tutorial progresses, you'll see details of how this trait is used and why. This is enough info to help you understand what I'm Deriving toward.

## Table of Contents

- [01 - Create the project](#chapter_01)
- [02 - Define the CacheDiff trait manually](#chapter_02)
- [03 - Create an empty Proc Macro](#chapter_03)
- [04 - Create a ParseField to Derive with](#chapter_04)
- [05 - Create a ParseContainer to Derive with](#chapter_05)
- [06 - Implement the basic Derive macro](#chapter_06)
- [07 - Understanding attributes for Derive customization](#chapter_07)
- [08 - Add attributes to ParseField](#chapter_08)
- [09 - Add attributes to ParseContainer](#chapter_09)
- [10 - Implement the full Derive macro (customizable with attributes)](#chapter_10)

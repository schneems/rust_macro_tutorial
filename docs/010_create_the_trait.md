<span id="chapter_01" />

## 01: Create the project

We will begin by creating two projects: one to hold our trait definition and one to hold our "derive" proc macro.

```term
:::>> $ mkdir cache_diff
:::>> $ cd cache_diff
```

Initialize two projects:

```term
:::>- $ cargo init cache_diff --lib
:::>- $ cargo init cache_diff_derive --lib
```

Tell Rust that these two projects live under one unified workspace by creating a `Cargo.toml` in the root (one directory above the projects you just made).

```toml
:::>> file.write "Cargo.toml"
[workspace]
members = [
    "cache_diff",
    "cache_diff_derive"
]
resolver = "2"
```

This workspace will allow us to run all tests via `cargo test` from the top-level directory. Ignore your build files so they don't appear in the debug output later.

```
:::>> file.write .gitignore
target/
```

```
:::-- $ echo ".DS_Store" >> .gitignore
:::-- $ echo needed for exa --git-ignore to function properly
:::-- $ git init
```

The project now looks like this:

```term
:::>> $ exa --tree --git-ignore .
```

We need two crates because a proc-macro must live in a stand-alone crate. This split allows Rust to compile and run that code before the rest of the code in a project is compiled. A limitation is that it can only export macros, so we need somewhere else for other public things (like traits) to live.

<span id="chapter_02" />

## Define the CacheDiff trait manually

Once the project is set up, we'll start by defining a public trait:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", code: <<-CODE)
pub trait CacheDiff {
    fn diff(&self, old: &Self) -> Vec<String>;
}
CODE
%>
```

This trait is short. It's designed to communicate that a struct is intended to be used as a cache key. You'll see how it's used in some tests below.

Fundamentally, macros are a form of metaprogramming: code that writes code. Before jumping into any kind of automation, you'll want to understand how to do the task manually first. I try to iterate as much as possible manually before solidifying things in macros.

## Manually implement the trait

> [Skip](#chapter_02) the rest of this section if: You already understand how the trait interface could be used and could write your tests for it.

Here's a test showing how a developer might manually implement this trait. First, we will add a "stringly" typed `Metadata` struct and implement the `CacheDiff` trait to simulate a world where we're storing a version of an architecture-dependent binary that we're installing:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", test_use: "    use super::*;", test_code: <<-CODE)
    struct Metadata {
        ruby_version: String,
        architecture: String,
    }

    impl CacheDiff for Metadata {
        fn diff(&self, old: &Self) -> Vec<String> {
            let mut diff = Vec::new();

            if self.ruby_version != old.ruby_version {
                diff.push(format!("ruby version ({} to {})",
                old.ruby_version,
                self.ruby_version))
            }
            if self.architecture != old.architecture {
                diff.push(
                    format!("architecture ({} to {})",
                    old.architecture,
                    self.architecture)
                )
            }
            diff
        }
    }
CODE
%>
```

With that definition out of the way, we can assert that the interface behaves as expected. Add this test case:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", test_code: <<-CODE)
    #[test]
    fn test_changed_metadata() {
        let old = Metadata {
            ruby_version: "3.3.1".to_string(),
            architecture: "amd64".to_string()
        };
        let new = Metadata {
            ruby_version: "3.4.2".to_string(),
            architecture: "arm64".to_string()
        };

        assert_eq!(
            vec![
                "ruby version (3.3.1 to 3.4.2)".to_string(),
                "architecture (amd64 to arm64)".to_string()
            ],
            new.diff(&old)
        );
    }
CODE
%>
```

It's usually a good idea to assert both positive and negative behavior. Add this test case:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", test_code: <<-CODE)
    #[test]
    fn test_unchanged_metadata() {
        let old = Metadata {
            ruby_version: "3.1.4".to_string(),
            architecture: "amd64".to_string()
        };

        let diff = old.diff(&old);
        assert!(
            diff.is_empty(),
            "Expected diff to be empty but is {:?}",
            diff
        );
    }
CODE
%>
```

Your file should now look like this:

```rust
:::-> $ cat cache_diff/src/lib.rs
```

And when you run tests, it should look a little like this:

```
:::>> $ cargo test
```

## Why Derive when we can walk?

The `CacheDiff` trait isn't too complicated to implement manually, but the code is repetitive. There's also room to mess up the output, like inverting the version number position or comparing one field and displaying values for a different one.

A derive macro would reduce repetition and eliminate silly logic errors while providing sensible defaults.

## Create the project

Create a top level workspace directory

```term
:::>> $ mkdir cache_diff
:::>> $ cd cache_diff
```

Now initialize two projects:

```
:::>> $ cargo init cache_diff --lib
:::>> $ cargo init cache_diff_derive --lib
```

Tell Rust that these two projects live under one unified workspace by creating a `Cargo.toml` in the root (one directory above the projects you just made):

```
:::-> file.write "Cargo.toml"
[workspace]
members = [
    "cache_diff",
    "cache_diff_derive"
]
resolver = "2"
```

```
:::>> file.write .gitignore
/target
.DS_Store
```

The project now looks like this:

```
:::>> $ exa --tree --git-ignore .
```

We need two crates because a procmacro must live in a stand alone crate. This allows Rust to compile and run that code before the rest of the code in a project is compiled. A limitation is that it can only export macros, so we need somewhere else for other public things (like traits) to live.

## Define the CacheDiff trait

Once the project is setup, we'll start off by defining a public trait:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", code: <<-CODE)
pub trait CacheDiff {
    fn diff(&self, old: &Self) -> Vec<String>;
}
CODE
%>
```

>note
>This is how I recommend starting: define a manual workflow first and once you're happy with that, then move on to automation/metaprogramming via proc-macro.

This trait is short. It's designed to communicate that a struct is intended to be used as a cache key. When compared to an older version of the struct, it should return an empty `Vec` if there are no differences (and the cache should be preserved). When the cache should be cleared, the entries represent list of human readable reasons why the cache was cleared (what is different between the two structs). The primary use case is that "metadata" structs are serialized to TOML to know when we can invalidate a layer in a [Cloud Native Buildpack (CNB) written in Rust with the libcnb.rs](https://github.com/heroku/libcnb.rs).

## Manually implement the trait

Without a macro, a maintainer would need to manually implement the trait, here's a test demonstrating what that would look like.

First we will add a stringly typed `Metadata` struct and implement `CacheDiff` for this struct to simulate a world where we're storing a version of an architecture dependent binary that we're installing:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", test_use: "    use super::*;\n", test_code: <<-CODE)
    struct Metadata {
        version: String,
        architecture: String,
    }

    impl CacheDiff for Metadata {
        fn diff(&self, old: &Self) -> Vec<String> {
            let mut diff = Vec::new();

            if self.version != old.version {
                diff.push(format!("version (`{}` to `{}`)",
                old.version,
                self.version))
            }
            if self.architecture != old.architecture {
                diff.push(
                    format!("architecture (`{}` to `{}`)",
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

> Skip the rest of this section if: You already understand how the trait interface could be used and could write your own tests for it.

Now, add a test for this behavior:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", test_code: <<-CODE)
    #[test]
    fn test_changed_metadata() {
        let old = Metadata {
            version: "3.1.4".to_string(),
            architecture: "amd64".to_string()
        };
        let new = Metadata {
            version: "3.5.0".to_string(),
            architecture: "arm64".to_string()
        };

        assert_eq!(
            vec![
                "version (`3.1.4` to `3.5.0`)".to_string(),
                "architecture (`amd64` to `arm64`)".to_string()
            ],
            new.diff(&old)
        );
    }
CODE
%>
```

It's usually a good idea to assert both positive an negative behavior:

```rust
:::>> print.erb
<%= append(filename: "cache_diff/src/lib.rs", test_code: <<-CODE)
    #[test]
    fn test_unchanged_metadata() {
        let old = Metadata {
            version: "3.1.4".to_string(),
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

The `CacheDiff` trait isn't too complicated, but there's a lot of repetition since the text of the output matches the field names (i.e. `metadata.version`), and there's room to mess up the output like inverting the version number position or comparing one field and displaying values for a different one.

If only there was a way to reduce repetition and eliminate silly logic errors. Some kind of code that could reflect on the struct we want to write and generate that output for us. Lucky for us, there is! That's what we'll work towards next.

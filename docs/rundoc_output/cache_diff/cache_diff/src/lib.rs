// File: `cache_diff/src/lib.rs`
//! Cache Diff (derive)
//!
//! Generate the difference between two structs for the purposes of cache invalidation.
//!
//! Example:
//!
//! ```
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
//! ```

//! ## Ignore attributes
//!
//! If the struct contains fields that should not be included in the diff comparison, you can ignore them:
//!
//! ```rust
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
//! ```

//! ## Rename attributes
//!
//! If your field name is not descriptive enough, you can rename it:
//!
//! ```rust
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
//! ```
//!

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
//! ```rust
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
//! ```
//!

//! ## Customize one or more field differences
//!
//! You can provide a custom implementation for a diffing a subset of fields without having to roll your own implementation.
//!
//! ### Custom logic for one field example
//!
//! Here's an example where someone wants to bust the cache after N cache calls. Everything else other than `cache_usage_count` can be derived. If you want to keep the existing derived difference checks, but add on a custom one you can do it like this:
//!
//! ```rust
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
//! ```
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

#[cfg(feature = "derive")]
pub use cache_diff_derive::CacheDiff;

// Code
pub trait CacheDiff {
    fn diff(&self, old: &Self) -> Vec<String>;
}

#[cfg(test)]
mod tests {
    // Test use
    use super::*;
    // Test code
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
}

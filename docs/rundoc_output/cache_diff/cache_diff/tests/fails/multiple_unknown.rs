use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct CustomDiffFn {
    #[cache_diff(unknown)]
    #[cache_diff(unknown = "value")]
    #[cache_diff(unknown = function)]
    name: String,
}

fn main() {}

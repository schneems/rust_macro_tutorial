use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct Metadata {
    #[cache_diff(ignore)]
    #[cache_diff(rename = "value")]
    name: String,

    #[cache_diff(rename = "value", ignore)]
    title: String,
}

fn main() {}

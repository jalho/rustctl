pub fn fmt_source_tree<E>(error: &E) -> String
where
    E: std::error::Error,
{
    let mut concatenated: String = String::new();

    concatenated.push_str(&format!("{error}"));

    let mut source: Option<_> = error.source();
    while let Some(src) = source {
        concatenated.push_str(&format!(": {src}"));
        source = src.source();
    }

    concatenated
}

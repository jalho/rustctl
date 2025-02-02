//! Miscellaneus utilities.

pub fn aggregate_error_tree<Error: std::error::Error + 'static>(
    error: &Error,
    indent_step: usize,
) -> String {
    let mut next: Option<&(dyn std::error::Error)> = Some(error);
    let mut gen: usize = 0;
    let mut aggregated: String = String::new();
    while let Some(node) = next {
        let prefix_len: usize = gen * indent_step;
        let mut indent: String = String::with_capacity(prefix_len);
        for _ in 0..prefix_len {
            indent.push(' ');
        }
        aggregated.push_str(&indent);
        aggregated.push_str(&format!("{}", node));
        aggregated.push('\n');
        next = node.source();
        gen = gen + 1;
    }
    return aggregated;
}

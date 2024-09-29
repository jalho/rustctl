pub static HELPTEXT: &'static str = r#"SYNOPSIS

    Tooling for running a Rust (the game) server and an integrated web service
    on Linux.

EXAMPLES

    rustctl --help
    rustctl --version
    rustctl config init
    rustctl config show
    rustctl web start
    rustctl game start
    rustctl health start"#;
pub static INFOTEXT: &'static str =
    concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

#[derive(clap::Parser)]
pub struct Cli {
    #[arg(short, long, default_value = "info", value_parser = parse_log_level)]
    pub log_level: log::LevelFilter,
    #[command(subcommand)]
    pub subcommand: Subcommand,
}

fn parse_log_level(input: &str) -> std::result::Result<log::LevelFilter, std::string::String> {
    const SUPPORTED_LEVELS: [(&str, log::LevelFilter); 6] = [
        ("off", log::LevelFilter::Off),
        ("error", log::LevelFilter::Error),
        ("warn", log::LevelFilter::Warn),
        ("info", log::LevelFilter::Info),
        ("debug", log::LevelFilter::Debug),
        ("trace", log::LevelFilter::Trace),
    ];

    SUPPORTED_LEVELS
        .iter()
        .find(|(name, _)| {
            let name: &str = *name;
            name == input
        })
        .map(|&(_, level)| level)
        .ok_or_else(|| {
            let supported = SUPPORTED_LEVELS
                .iter()
                .map(|(name, _)| *name)
                .collect::<std::vec::Vec<&str>>()
                .join(", ");
            format!("supported values: {supported}")
        })
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
    GameStart {
        #[arg(
            long,
            help = "Exclude a directory from the game start process's search for the game executable.",
            long_help = r#"Exclude a directory from the game start process's search for the game
executable. This is useful, for example, when developing on WSL (Windows
Subsystem for Linux), where performing a whole system wide search tends to be
particularly slow. In such cases, you may want to exclude `/mnt/c/`"#,
            value_name = "DIRECTORY",
            default_value = None
        )]
        exclude: Option<std::path::PathBuf>,
    },
}

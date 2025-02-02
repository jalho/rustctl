pub fn parse_buildid_from_manifest(manifest_path: &std::path::Path) -> Option<u32> {
    if let Ok(content) = std::fs::read_to_string(manifest_path) {
        for line in content.lines() {
            let trimmed: &str = line.trim();
            if trimmed.starts_with("\"buildid\"") {
                if let Some(_) = trimmed.find('\"') {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(buildid) = parts[1].trim_matches('"').parse::<u32>() {
                            return Some(buildid);
                        }
                    }
                }
            }
        }
    }
    return None;
}

#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Subcommand,
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

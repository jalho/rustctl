mod core;
mod logging;
mod system;
mod util;

static EXIT_OK: u8 = 0;
static EXIT_ERR_LOGGER: u8 = 42;
static EXIT_ERR_OTHER: u8 = 43;

fn main() -> std::process::ExitCode {
    let cli: crate::parsers::Cli = <crate::parsers::Cli as clap::Parser>::parse();

    let _handle: log4rs::Handle = match crate::logging::init_logger() {
        Ok(n) => n,
        Err(err) => {
            eprintln!("{}", crate::util::aggregate_error_tree(&err, 2));
            return std::process::ExitCode::from(EXIT_ERR_LOGGER);
        }
    };

    match cli.subcommand {
        crate::parsers::Subcommand::GameStart { exclude } => {
            let game: crate::core::Game = match crate::core::Game::start(exclude) {
                Ok(n) => n,
                Err(err) => {
                    /* TODO:
                     * Check if error case works: "Running parallel" (Multiple
                     * processes called "RustDedicated" already running)
                     */
                    log::error!(
                        "Cannot start game: {}",
                        crate::util::aggregate_error_tree(&err, 2)
                    );
                    return std::process::ExitCode::from(EXIT_ERR_OTHER);
                }
            };
            log::info!("Game started: {game}");
        }
    }

    return std::process::ExitCode::from(EXIT_OK);
}

mod parsers {
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
}

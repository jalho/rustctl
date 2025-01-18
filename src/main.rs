mod args;
mod error;
mod misc;
mod proc;

static EXIT_OK: i32 = 0;
static EXIT_ERR_DEPENDENCY_MISSING: i32 = 42;

fn main() {
    _ = crate::misc::init_logger();

    let cli: crate::args::RustCtlCli = clap::Parser::parse();
    match cli.command {
        crate::args::CliCommand::Game { subcommand: action } => match action {
            crate::args::CliSubCommandGame::InstallUpdateConfigureStart { skip_install } => {
                let steamcmd_cli: crate::proc::Dependency =
                    match crate::proc::Dependency::init("steamcmd") {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!("Unrecoverable error: {}", err);
                            std::process::exit(EXIT_ERR_DEPENDENCY_MISSING);
                        }
                    };
            }
        },
    }

    std::process::exit(EXIT_OK);
}

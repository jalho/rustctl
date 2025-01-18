mod args;

fn main() {
    let cli: crate::args::RustCtlCli = clap::Parser::parse();

    match cli.command {
        crate::args::CliCommand::Game { subcommand: action } => match action {
            crate::args::CliSubCommandGame::InstallUpdateConfigureStart { skip_install } => {
                println!(
                    "Game start command executed. --skip_install: {}",
                    skip_install
                );
            }
        },
    }
}

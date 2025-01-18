mod args;
mod proc;

fn main() {
    let cli: crate::args::RustCtlCli = clap::Parser::parse();

    match cli.command {
        crate::args::CliCommand::Game { subcommand: action } => match action {
            crate::args::CliSubCommandGame::InstallUpdateConfigureStart { skip_install } => {
                let steamcmd_cli: crate::proc::Dependency =
                    crate::proc::Dependency::init("steamcmd");
            }
        },
    }
}

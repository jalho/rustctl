mod ci;
mod deploy;
mod dist;
mod init;

fn main() -> Result<(), std::process::ExitCode> {
    let cli: init::Cli = init::Cli::get();
    let _logg: log4rs::Handle = init::init_logger()?;

    match cli.command {
        None | Some(init::Command::Ci) => {
            ci::check_format_lint()?;
        }

        Some(init::Command::Dist) => {
            ci::check_format_lint()?;
            dist::build_release_deb()?;
        }

        Some(init::Command::Deploy) => {
            deploy::via_ssh()?;
        }
    }

    Ok(())
}

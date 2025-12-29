mod ci;
mod init;

fn main() -> Result<(), std::process::ExitCode> {
    let cli: init::Cli = init::Cli::get();
    let _logg: log4rs::Handle = init::init_logger()?;

    match cli.command {
        None | Some(init::Command::Ci) => {
            ci::check_format_lint()?;
        }
    }

    Ok(())
}

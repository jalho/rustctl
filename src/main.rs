mod args;
mod error;
mod misc;
mod text;

/* TODO: Make descriptive error log everywhere where FatalError is bubbled from:
Simplify it by only having it contain that message. */

fn main() -> Result<(), error::FatalError> {
    misc::init_logger()?;
    log::debug!("Logger initialized");

    let argv: Vec<String> = std::env::args().collect();
    let config: args::Config = args::Config::get_from_fs(args::Config::default_fs_path())?;

    match args::Command::get(argv)? {
        args::Command::Config => todo!(),
        args::Command::GameStart => {
            misc::install_steamcmd(
                &config.steamcmd_download_url,
                &config.rustctl_root_dir,
                &config.steamcmd_target_file_name_tgz,
                &config.steamcmd_executable_name,
            )?;
        }
        args::Command::HealthStart => todo!(),
        args::Command::Help => {
            println!("{}", text::HELPTEXT);
        }
        args::Command::Version => {
            println!("{}", text::INFOTEXT);
        }
        args::Command::WebStart => todo!(),
    };

    return Ok(());
}

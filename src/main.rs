mod args;

fn main() -> Result<(), args::ArgError> {
    let argv: Vec<String> = std::env::args().collect();
    let config: args::Config = args::Config::get_from_fs(args::Config::default_fs_path())?;

    match args::Command::get(argv)? {
        args::Command::GameStart => {
            println!(
                "TODO: Download SteamCMD from {} to {:?}",
                config.download_url_steamcmd, config.rustctl_root_dir
            );
        }
        args::Command::HealthStart => todo!(),
        args::Command::WebStart => todo!(),
    };

    return Ok(());
}

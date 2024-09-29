mod args;
mod http;
mod text;

enum FatalError {
    ArgError(args::ArgError),
    HttpError(http::HttpError),
}
impl std::fmt::Debug for FatalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgError(arg0) => f.debug_tuple("ArgError").field(arg0).finish(),
            Self::HttpError(arg0) => f.debug_tuple("HttpError").field(arg0).finish(),
        }
    }
}
impl From<args::ArgError> for FatalError {
    fn from(err: args::ArgError) -> Self {
        return Self::ArgError(err);
    }
}
impl From<http::HttpError> for FatalError {
    fn from(err: http::HttpError) -> Self {
        return Self::HttpError(err);
    }
}

fn main() -> Result<(), FatalError> {
    let argv: Vec<String> = std::env::args().collect();
    let config: args::Config = args::Config::get_from_fs(args::Config::default_fs_path())?;

    match args::Command::get(argv)? {
        args::Command::Config => todo!(),
        args::Command::GameStart => {
            /* TODO: Only download SteamCMD if necessary */
            let _ = download_steamcmd(config.download_url_steamcmd, &config.rustctl_root_dir)?;
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

/// Download _SteamCMD_ (game server installer).
fn download_steamcmd(
    url: String,
    download_dir: &std::path::PathBuf,
) -> Result<(), http::HttpError> {
    let mut response: std::net::TcpStream = http::request(url)?;
    /* TODO: Extract the .tgz */
    /* TODO: Assert expected entry point exists (steamcmd.sh or something) */
    let mut download_dir = download_dir.clone();
    download_dir.push("steamcmd.tgz");
    http::stream_to_disk(&mut response, &download_dir)?;
    return Ok(());
}

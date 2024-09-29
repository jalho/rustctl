mod args;
mod text;

enum FatalError {
    ArgError(args::ArgError),
    HttpError(HttpError),
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
impl From<HttpError> for FatalError {
    fn from(err: HttpError) -> Self {
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

enum HttpError {
    NoDelimiter,
    BadUrl(String), /* TODO: Check this at config validation instead? */
    IO(std::io::ErrorKind),
}
impl std::fmt::Debug for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDelimiter => write!(f, "NoDelimiter"),
            Self::IO(arg0) => f.debug_tuple("IO").field(arg0).finish(),
            Self::BadUrl(arg0) => f.debug_tuple("BadUrl").field(arg0).finish(),
        }
    }
}
impl From<std::io::Error> for HttpError {
    fn from(err: std::io::Error) -> Self {
        return Self::IO(err.kind());
    }
}

fn download_steamcmd(url: String, download_dir: &std::path::PathBuf) -> Result<(), HttpError> {
    let (host, path): (&str, &str) =
        match url.strip_prefix("http://").and_then(|u| u.split_once('/')) {
            Some((n, m)) => (n, m),
            None => {
                return Err(HttpError::BadUrl(format!(
                    "expected HTTP URL with path, got '{}'",
                    url
                )));
            }
        };
    let mut stream: std::net::TcpStream = std::net::TcpStream::connect(host)?;
    let buf_out: String =
        format!("GET /{path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n",);
    use std::io::Write;
    stream.write_all(buf_out.as_bytes())?;

    /* TODO: Stream the response to disk */
    /* TODO: Extract the .tgz */
    /* TODO: Assert expected entry point exists (steamcmd.sh or something) */
    let mut buf_in = Vec::new();
    use std::io::Read;
    stream.read_to_end(&mut buf_in)?;

    let headers_end: usize = match buf_in.windows(4).position(|window| window == b"\r\n\r\n") {
        Some(pos) => pos,
        None => {
            return Err(HttpError::NoDelimiter);
        }
    };
    let headers = &buf_in[..headers_end];
    let body = &buf_in[headers_end + 4..];
    let content_length = parse_content_length(headers).unwrap_or(body.len());
    let payload = &body[..content_length];

    let mut download_dir = download_dir.clone();
    download_dir.push("steamcmd.tgz");
    std::fs::write(download_dir, payload)?;
    return Ok(());
}

fn parse_content_length(headers: &[u8]) -> Option<usize> {
    let headers_str = String::from_utf8_lossy(headers);
    for line in headers_str.lines() {
        if line.to_lowercase().starts_with("content-length:") {
            if let Some(length) = line.split(':').nth(1) {
                return length.trim().parse().ok();
            }
        }
    }
    None
}

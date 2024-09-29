mod args;
mod text;

fn main() -> Result<(), args::ArgError> {
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

fn download_steamcmd(url: String, download_dir: &std::path::PathBuf) -> Result<(), args::ArgError> {
    let (host, path): (&str, &str) =
        match url.strip_prefix("http://").and_then(|u| u.split_once('/')) {
            Some((n, m)) => (n, m),
            None => {
                return Err(args::ArgError::ConfigInvalid(format!(
                    "expected HTTP URL with path, got: '{}'",
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
        /* TODO: Add fatal error case */
        None => todo!(),
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

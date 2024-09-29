mod args;

fn main() -> Result<(), args::ArgError> {
    let argv: Vec<String> = std::env::args().collect();
    let config: args::Config = args::Config::get_from_fs(args::Config::default_fs_path())?;

    match args::Command::get(argv)? {
        args::Command::Config => todo!(),
        args::Command::GameStart => {
            println!(
                "TODO: Download SteamCMD from {} to {:?}",
                config.download_url_steamcmd, config.rustctl_root_dir
            );
            let _ = download_steamcmd();
        }
        args::Command::HealthStart => todo!(),
        args::Command::Version => {
            let package_name = env!("CARGO_PKG_NAME");
            let version = env!("CARGO_PKG_VERSION");
            println!("{} v{}", package_name, version);
        }
        args::Command::WebStart => todo!(),
    };

    return Ok(());
}

fn download_steamcmd() -> Result<(), std::io::Error> {
    let url: &str = "127.0.0.1:8080";
    let path: &str = "/steamcmd.tgz";
    let mut stream: std::net::TcpStream = std::net::TcpStream::connect(url)?;

    let buf_out: String = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, url
    );
    use std::io::Write;
    stream.write_all(buf_out.as_bytes())?;

    /* TODO: Stream the response to disk. */
    let mut buf_in = Vec::new();
    use std::io::Read;
    stream.read_to_end(&mut buf_in)?;

    let headers_end: usize = match buf_in.windows(4).position(|window| window == b"\r\n\r\n") {
        Some(pos) => pos,
        None => todo!(),
    };
    let headers = &buf_in[..headers_end];
    let body = &buf_in[headers_end + 4..];
    let content_length = parse_content_length(headers).unwrap_or(body.len());
    let payload = &body[..content_length];

    std::fs::write("steamcmd.tgz", payload)?;
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

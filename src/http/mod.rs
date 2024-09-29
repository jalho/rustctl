//! HTTP stuff.

/// Failures related to HTTP operations.
pub enum HttpError {
    BadUrl(String),
    IO(std::io::ErrorKind),
    HeaderDelimiterError(String),
}
impl std::fmt::Debug for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(arg0) => f.debug_tuple("IO").field(arg0).finish(),
            Self::BadUrl(arg0) => f.debug_tuple("BadUrl").field(arg0).finish(),
            Self::HeaderDelimiterError(arg0) => {
                f.debug_tuple("HeaderDelimiterError").field(arg0).finish()
            }
        }
    }
}
impl From<std::io::Error> for HttpError {
    fn from(err: std::io::Error) -> Self {
        return Self::IO(err.kind());
    }
}

/// Send an HTTP request.
pub fn request(url: &String) -> Result<std::net::TcpStream, HttpError> {
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
    return Ok(stream);
}

/// Stream an HTTP response payload to disk.
pub fn stream_to_disk<R: std::io::Read>(
    mut stream: R,
    download_dir: &std::path::PathBuf,
) -> std::result::Result<usize, HttpError> {
    let mut buffer: [u8; 8192] = [0; 8192];
    let mut buffer_headers: Vec<u8> = Vec::new();
    let delimiter: &[u8; 4] = b"\r\n\r\n";

    let mut file_out: std::fs::File = std::fs::File::create(download_dir)?;
    let mut total_bytes_written: usize = 0;

    /* TODO: Wait for headers delimiter only up till some threshold? */
    loop {
        let bytes_read: usize = stream.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        buffer_headers.extend_from_slice(&buffer[..bytes_read]);

        // only write data that follows headers...
        if let Some(i) = buffer_headers.windows(4).position(|n| n == delimiter) {
            let body_start: usize = i + delimiter.len();
            use std::io::Write;
            file_out.write_all(&buffer_headers[body_start..])?;
            total_bytes_written += buffer_headers.len() - body_start;

            // ...and then write the rest of the data till the end
            while let Ok(bytes_read) = stream.read(&mut buffer) {
                if bytes_read == 0 {
                    break;
                }
                file_out.write_all(&buffer[..bytes_read])?;
                total_bytes_written += bytes_read;
            }
            break;
        }
    }

    return Ok(total_bytes_written);
}

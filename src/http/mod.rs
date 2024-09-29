//! HTTP stuff.

/// Failures related to HTTP operations.
pub enum HttpError {
    BadUrl(String),
    IO(std::io::ErrorKind),
    HeaderDelimiterError(String),
    EmptyPayload,
}
impl std::fmt::Debug for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(arg0) => f.debug_tuple("IO").field(arg0).finish(),
            Self::BadUrl(arg0) => f.debug_tuple("BadUrl").field(arg0).finish(),
            Self::HeaderDelimiterError(arg0) => {
                f.debug_tuple("HeaderDelimiterError").field(arg0).finish()
            }
            Self::EmptyPayload => f.debug_tuple("EmptyPayload").finish(),
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
    target_path: &std::path::PathBuf,
) -> std::result::Result<usize, HttpError> {
    let mut buffer: [u8; 8192] = [0; 8192];
    let mut buffer_headers: Vec<u8> = Vec::new();
    let delimiter: &[u8; 4] = b"\r\n\r\n";

    let mut file_out: std::fs::File = std::fs::File::create(target_path)?;
    let mut total_bytes_written: usize = 0;
    let mut total_bytes_read: usize = 0;

    /* TODO: Wait for headers delimiter only up till some threshold? */
    loop {
        let bytes_read: usize = stream.read(&mut buffer)?;
        total_bytes_read += bytes_read;
        if bytes_read == 0 {
            break;
        }
        buffer_headers.extend_from_slice(&buffer[..bytes_read]);

        let headers_size_max: usize = 8192;
        // only write data that follows headers...
        if let Some(i) = buffer_headers.windows(4).position(|n| n == delimiter) {
            let body_start: usize = i + delimiter.len();
            use std::io::Write;
            file_out.write_all(&buffer_headers[body_start..])?;
            total_bytes_written += buffer_headers.len() - body_start;

            // ...and then write the rest of the data till the end
            while let Ok(bytes_read) = stream.read(&mut buffer) {
                total_bytes_read += bytes_read;
                if bytes_read == 0 {
                    break;
                }
                file_out.write_all(&buffer[..bytes_read])?;
                total_bytes_written += bytes_read;
            }
            break;
        } else if total_bytes_read >= headers_size_max {
            return Err(HttpError::HeaderDelimiterError(format!(
                "header delimiter not found within the first {} bytes",
                headers_size_max
            )));
        }
    }

    if total_bytes_written < 1 {
        return Err(HttpError::EmptyPayload);
    } else {
        return Ok(total_bytes_written);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct MockStream {
        data: Vec<u8>,
        position: usize,
    }
    impl MockStream {
        fn new(data: &[u8]) -> Self {
            return Self {
                data: data.to_vec(),
                position: 0,
            };
        }
    }
    impl std::io::Read for MockStream {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
            if self.position >= self.data.len() {
                return Ok(0);
            }
            let bytes_to_read: usize = std::cmp::min(buf.len(), self.data.len() - self.position);
            buf[..bytes_to_read]
                .copy_from_slice(&self.data[self.position..self.position + bytes_to_read]);
            self.position += bytes_to_read;
            return Ok(bytes_to_read);
        }
    }

    #[test]
    fn test_stream_to_disk_headers_too_large() {
        let mock_data: [u8; 10000] = [0; 10000]; // No \r\n\r\n within the first 8192 bytes
        let stream = MockStream::new(&mock_data);
        let path = std::path::PathBuf::from("/dev/null");
        let result = stream_to_disk(stream, &path);
        assert!(result.is_err());
        if let Err(HttpError::HeaderDelimiterError(msg)) = result {
            assert_eq!(
                msg,
                "header delimiter not found within the first 8192 bytes"
            );
        } else {
            panic!("Expected HeaderDelimiterError");
        }
    }

    #[test]
    fn test_stream_to_disk_ok() {
        let mock_data = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\na";
        let stream = MockStream::new(mock_data);
        let path = std::path::PathBuf::from("/dev/null");
        let result = stream_to_disk(stream, &path);
        assert!(result.is_ok());
        match result {
            Ok(payload_bytes_received) => assert_eq!(payload_bytes_received, 1),
            Err(_) => panic!("expected payload_bytes_received: usize"),
        }
    }

    #[test]
    fn test_stream_to_disk_headers_delimiter_missing() {
        let mock_data = b"HTTP/1.1 200 OK\r\nContent-Length:"; // abrupt end without \r\n\r\n
        let stream = MockStream::new(mock_data);
        let path = std::path::PathBuf::from("/dev/null");
        let result = stream_to_disk(stream, &path);
        match result {
            Err(HttpError::EmptyPayload) => {}
            _ => panic!("expected error EmptyPayload"),
        }
    }
}

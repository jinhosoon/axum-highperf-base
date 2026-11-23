use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Inbound CLI Adapter: 컨테이너 자체 헬스체크 프로브 (`rust-api --healthcheck`)
pub fn run_healthcheck(port: u16) -> bool {
    let Ok(addr) = format!("127.0.0.1:{}", port).parse() else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_secs(2)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    if stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut buf = [0u8; 128];
    let Ok(n) = stream.read(&mut buf) else {
        return false;
    };
    std::str::from_utf8(&buf[..n])
        .map(|res| res.starts_with("HTTP/1.1 200"))
        .unwrap_or(false)
}

//! Connection helper — timeout-aware TCP to the kernel admin port.
//!
//! Stage-9 hardening:
//!   * Returns `CliError::Connection` instead of `String`, so the caller
//!     gets correct exit code mapping.
//!   * `--server` global flag overrides everything via `crate::config`.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::error::{CliError, CliResult};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const READ_TIMEOUT:    Duration = Duration::from_secs(15);
const WRITE_TIMEOUT:   Duration = Duration::from_secs(5);

pub struct Conn { stream: TcpStream }

impl Conn {
    pub fn open(server: &str) -> CliResult<Self> {
        let addr: std::net::SocketAddr = server.parse()
            .or_else(|_| server.to_socket_addrs_first())
            .map_err(|e| CliError::Connection(format!("resolve {}: {}", server, e)))?;

        let stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)
            .map_err(|e| CliError::Connection(format!("connect {}: {}", server, e)))?;
        stream.set_read_timeout(Some(READ_TIMEOUT)).ok();
        stream.set_write_timeout(Some(WRITE_TIMEOUT)).ok();
        Ok(Self { stream })
    }

    pub fn open_default() -> CliResult<Self> {
        let cfg = crate::config::load()?;
        Self::open(&cfg.server)
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> CliResult {
        self.stream.write_all(bytes)
            .map_err(|e| CliError::Connection(format!("write: {}", e)))
    }

    pub fn read_to_string(self) -> CliResult<String> {
        let mut buf = String::new();
        let mut s = self.stream;
        s.read_to_string(&mut buf)
            .map_err(|e| CliError::Connection(format!("read: {}", e)))?;
        Ok(buf)
    }

    pub fn for_each_line(self, mut on_line: impl FnMut(String) -> bool) -> CliResult {
        let r = BufReader::new(self.stream);
        for line in r.lines() {
            match line {
                Ok(l)  => if !on_line(l) { return Ok(()); }
                Err(e) => return Err(CliError::Connection(format!("read: {}", e))),
            }
        }
        Ok(())
    }
}

trait ToSocketAddrsFirst {
    fn to_socket_addrs_first(&self) -> std::io::Result<std::net::SocketAddr>;
}
impl ToSocketAddrsFirst for str {
    fn to_socket_addrs_first(&self) -> std::io::Result<std::net::SocketAddr> {
        use std::net::ToSocketAddrs;
        self.to_socket_addrs()?.next().ok_or_else(||
            std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "no addr"))
    }
}

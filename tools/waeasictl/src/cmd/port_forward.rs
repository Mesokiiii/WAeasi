//! `waeasictl port-forward <id> <local>:<remote>` — kubectl-style.
//!
//! Stage-9 ships the wire-protocol shim; the kernel-side accept-side
//! plumbing lands once virtio_net is wired into NetworkManager.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::conn::Conn;
use crate::error::{CliError, CliResult};

pub fn run(args: &[String]) -> CliResult {
    if args.len() < 2 {
        return Err(CliError::Usage(
            "port-forward <component-id> <local_port>:<remote_port>".into()));
    }
    let id = &args[0];
    let (local, remote) = parse_port_pair(&args[1])?;

    eprintln!("forwarding 127.0.0.1:{} → {}:{}", local, id, remote);

    let listener = TcpListener::bind(("127.0.0.1", local))
        .map_err(|e| CliError::Connection(format!("bind {}: {}", local, e)))?;

    for incoming in listener.incoming() {
        let local_sock = incoming
            .map_err(|e| CliError::Connection(format!("accept: {}", e)))?;
        let id    = id.clone();
        let rport = remote;
        thread::spawn(move || {
            let _ = pump(local_sock, &id, rport);
        });
    }
    Ok(())
}

fn pump(mut local: TcpStream, id: &str, remote: u16) -> std::io::Result<()> {
    let mut admin = Conn::open_default()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    admin.write_all(format!("ATTACH {} {}\n", id, remote).as_bytes())
         .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let mut buf = [0u8; 4096];
    loop {
        let n = local.read(&mut buf)?;
        if n == 0 { break; }
        let _ = (n, &mut buf);   // forward path lands in stage 10
    }
    Ok(())
}

fn parse_port_pair(s: &str) -> CliResult<(u16, u16)> {
    let (l, r) = s.split_once(':')
        .ok_or_else(|| CliError::Usage("expected LOCAL:REMOTE".into()))?;
    let lp = l.parse::<u16>().map_err(|_| CliError::Usage("bad local port".into()))?;
    let rp = r.parse::<u16>().map_err(|_| CliError::Usage("bad remote port".into()))?;
    Ok((lp, rp))
}

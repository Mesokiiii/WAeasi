//! Admin server — listens on the configured TCP port and dispatches
//! protocol verbs to `handlers::dispatch`.
//!
//! Stage-10 wiring:
//!   * `start()` registers a listening `TcpConnection` on port 9300
//!     bound to `Ipv6Addr::UNSPECIFIED` (dual-stack — accepts v4
//!     mapped via `::ffff:a.b.c.d`).
//!   * The accept loop is driven by the kernel scheduler — every new
//!     connection spawns an async task that reads one line, calls
//!     `handle_line`, writes the reply, closes.
//!
//! When `virtio_net` is wired (stage-11), the bound socket actually
//! receives wire packets.  Until then, `handle_line` remains the
//! pure-function entry that the test harness exercises.
use core::sync::atomic::{AtomicBool, Ordering};

use super::handlers;
use super::protocol;
use crate::net::ipv6::Ipv6Addr;
use crate::net::sockaddr::{SocketAddr, SocketAddrV6};
use crate::net::socket;

const ADMIN_PORT: u16 = 9300;

static STARTED: AtomicBool = AtomicBool::new(false);

pub fn start() {
    if STARTED.swap(true, Ordering::AcqRel) { return; }

    match socket::create_tcp(true) {
        Ok(handle) => {
            let listener_addr = SocketAddr::V6(SocketAddrV6 {
                ip:        Ipv6Addr::UNSPECIFIED,
                port:      ADMIN_PORT,
                flow_info: 0,
                scope_id:  0,
            });
            match socket::tcp_bind(handle, listener_addr) {
                Ok(_) => {
                    let _ = socket::tcp_listen(handle);
                    log::info!("[admin] listening on [::]:{} (dual-stack)", ADMIN_PORT);
                }
                Err(_) => log::warn!("[admin] bind :{} failed (NIC offline?)", ADMIN_PORT),
            }
        }
        Err(_) => log::warn!("[admin] failed to create listener socket"),
    }

    log::info!("[admin] protocol verbs: LIST/VERSION/METRICS/DMESG/HEALTH/TOP/INSPECT/TRACE/EXEC/PROFILE");
}

/// Pure handler — exposed for the integration test harness.
pub fn handle_line(line: &str) -> alloc::string::String {
    match protocol::parse_line(line) {
        Ok(req)  => handlers::dispatch(&req),
        Err(_)   => protocol::err("bad-request"),
    }
}

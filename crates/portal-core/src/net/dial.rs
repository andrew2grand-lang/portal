//! Outgoing TCP connections that leave through the right interface.
//!
//! - IPv6 link-local: the address carries our interface index as scope id.
//! - IPv4 169.254/16: macOS only routes link-local IPv4 out of the primary
//!   interface, so with Wi-Fi up a cable dial could leave over Wi-Fi. We bind
//!   the source address to the cable interface (and set IP_BOUND_IF on macOS)
//!   instead of trusting the routing table.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use socket2::{Domain, Protocol, SockRef, Socket, TcpKeepalive, Type};
use tokio::net::{TcpSocket, TcpStream};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalBind {
    pub ip: IpAddr,
    pub if_index: u32,
}

pub async fn dial(addr: SocketAddr, local: Option<LocalBind>, timeout: Duration) -> io::Result<TcpStream> {
    let socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
    socket.set_nonblocking(true)?;
    if let Some(bind) = local {
        #[cfg(target_vendor = "apple")]
        {
            let index = std::num::NonZeroU32::new(bind.if_index);
            if addr.is_ipv4() {
                socket.bind_device_by_index_v4(index)?
            } else {
                socket.bind_device_by_index_v6(index)?
            }
        }
        socket.bind(&SocketAddr::new(bind.ip, 0).into())?;
    }
    let std_stream: std::net::TcpStream = socket.into();
    let stream = tokio::time::timeout(timeout, TcpSocket::from_std_stream(std_stream).connect(addr))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, format!("no answer from {addr}")))??;
    tune(&stream)?;
    Ok(stream)
}

/// Low latency for control messages, keepalives so a vanished peer is noticed.
pub fn tune(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let keepalive = TcpKeepalive::new().with_time(Duration::from_secs(10)).with_interval(Duration::from_secs(5));
    SockRef::from(stream).set_tcp_keepalive(&keepalive)
}

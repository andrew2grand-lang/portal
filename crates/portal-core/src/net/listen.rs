//! TCP listeners on IPv4 and IPv6 (incl. link-local), same port on both.

use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::{TcpListener, TcpStream};

pub struct Listeners {
    /// The port actually bound (advertised via discovery).
    pub port: u16,
    v4: Option<TcpListener>,
    v6: Option<TcpListener>,
}

fn listener(addr: SocketAddr) -> io::Result<TcpListener> {
    let socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
    if addr.is_ipv6() {
        socket.set_only_v6(true)?;
    }
    // Unix: allow a quick restart while old connections sit in TIME_WAIT.
    // Not on Windows, where SO_REUSEADDR lets another process steal the port.
    #[cfg(unix)]
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.listen(128)?;
    socket.set_nonblocking(true)?;
    TcpListener::from_std(socket.into())
}

/// Binds `preferred` (0 = any). If it's taken, falls back to a random free
/// port. `loopback_only` is for tests: 127.0.0.1 only, no firewall prompts.
pub async fn bind(preferred: u16, loopback_only: bool) -> io::Result<Listeners> {
    if loopback_only {
        let v4 = listener(SocketAddr::from((Ipv4Addr::LOCALHOST, preferred)))?;
        return Ok(Listeners { port: v4.local_addr()?.port(), v4: Some(v4), v6: None });
    }
    let v4 = match listener(SocketAddr::from((Ipv4Addr::UNSPECIFIED, preferred))) {
        Ok(l) => l,
        Err(e) if e.kind() == io::ErrorKind::AddrInUse && preferred != 0 => {
            tracing::warn!("port {preferred} is taken; using a random port");
            listener(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))?
        }
        Err(e) => return Err(e),
    };
    let port = v4.local_addr()?.port();
    let v6 = listener(SocketAddr::from((Ipv6Addr::UNSPECIFIED, port)))
        .inspect_err(|e| tracing::warn!("no IPv6 listener on port {port}: {e}"))
        .ok();
    Ok(Listeners { port, v4: Some(v4), v6 })
}

impl Listeners {
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        match (&self.v4, &self.v6) {
            (Some(a), Some(b)) => tokio::select! { r = a.accept() => r, r = b.accept() => r },
            (Some(a), None) => a.accept().await,
            (None, Some(b)) => b.accept().await,
            (None, None) => std::future::pending().await,
        }
    }
}

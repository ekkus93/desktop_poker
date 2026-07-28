use std::{
    net::{Shutdown, TcpStream},
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

use super::*;

impl HostServer {
    /// Stop accepting reconnects and close every established client socket.
    ///
    /// Host client-session threads are detached and own the original
    /// `TcpStream`. The registry holds clones of those same sockets, so shutting
    /// down the clones wakes both the host-side and remote client-side blocking
    /// reads. The method is intentionally idempotent because `DesktopHostSession`
    /// requests shutdown before the final `HostServer` drop performs its normal
    /// thread joins.
    pub(crate) fn request_shutdown(&self) {
        self.stop_signal.store(true, Ordering::SeqCst);

        // Wake the blocking accept loop. A refused connection means the listener
        // has already stopped, which is also a successful shutdown state.
        if let Err(error) = TcpStream::connect(self.listener_addr) {
            if error.kind() != std::io::ErrorKind::ConnectionRefused {
                eprintln!(
                    "[host-shutdown] failed to wake listener {}: {error}",
                    self.listener_addr
                );
            }
        }

        // Do not close established sockets until the listener stops accepting;
        // otherwise a client's automatic reconnect could race back into a host
        // that is in the process of shutting down.
        let deadline = Instant::now() + Duration::from_millis(500);
        while let Ok(stream) = TcpStream::connect(self.listener_addr) {
            drop(stream);
            if Instant::now() >= deadline {
                eprintln!(
                    "[host-shutdown] listener {} remained reachable during shutdown",
                    self.listener_addr
                );
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        // Drain before locking individual streams. Detached session threads
        // remove themselves from this registry after the shutdown-induced read
        // failure; holding the registry lock while closing streams would risk a
        // lock-order deadlock.
        let connected_clients = match self.clients.lock() {
            Ok(mut clients) => clients
                .drain()
                .map(|(_, client)| client)
                .collect::<Vec<_>>(),
            Err(poisoned) => {
                eprintln!(
                    "[host-shutdown] connected-client registry lock was poisoned; recovering"
                );
                poisoned
                    .into_inner()
                    .drain()
                    .map(|(_, client)| client)
                    .collect::<Vec<_>>()
            }
        };

        for client in connected_clients {
            let stream = match client.stream.lock() {
                Ok(stream) => stream,
                Err(poisoned) => {
                    eprintln!("[host-shutdown] client stream lock was poisoned; recovering");
                    poisoned.into_inner()
                }
            };
            if let Err(error) = stream.shutdown(Shutdown::Both) {
                if error.kind() != std::io::ErrorKind::NotConnected {
                    eprintln!("[host-shutdown] failed to close client socket: {error}");
                }
            }
        }
    }
}

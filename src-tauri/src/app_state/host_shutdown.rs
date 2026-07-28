use super::*;

impl Drop for DesktopHostSession {
    fn drop(&mut self) {
        self.host_server.request_shutdown();
    }
}

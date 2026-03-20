// Phase 5 (post-MVP): SSH tunneling via ssh2-rs.
//
// This module will replace the shell-based VNC_VIA_CMD tunnel in the
// classic viewer (vncviewer/vncviewer.cxx:596-615) with a native Rust
// SSH tunnel using the ssh2 crate.
//
// Architecture:
//   1. SshTunnel::new(gateway, remote_host, remote_port) opens an SSH
//      connection to the gateway host.
//   2. It requests a TCP/IP channel forward to remote_host:remote_port.
//   3. It binds a local TCP listener on a random port.
//   4. The VNC connection then targets localhost:<local_port>.
//   5. Data is proxied: local socket <-> SSH channel <-> remote VNC server.
//
// This avoids exposing VNC ports on the network and removes the dependency
// on the system ssh binary.

#[allow(dead_code)]
pub struct SshTunnel {
    pub local_port: u16,
    pub gateway: String,
    pub remote_host: String,
    pub remote_port: u16,
    // handle: Option<thread::JoinHandle<()>>,
}

#[allow(dead_code)]
impl SshTunnel {
    pub fn is_available() -> bool {
        // Will return true once ssh2-rs is added as a dependency
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_tunnel_not_available_yet() {
        assert!(!SshTunnel::is_available());
    }
}

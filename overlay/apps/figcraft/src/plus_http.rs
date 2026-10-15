//! FigCraft Plus: one HTTPS round trip for the Google Fonts list, over FigCraft's own TLS client
//! (rustls, Mozilla's root certificates, certificates always verified).

use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Mutex;
use std::time::Duration;

use figcraft_collab_net::tls;

/// The most a response may hold.
const MAX: usize = 80 << 20;

/// Send `request` to `host`:443 and return the whole response (the server closes the connection
/// after it: requests say `Connection: close`).
pub fn round_trip(host: &str, request: &[u8]) -> Result<Vec<u8>, String> {
    let addr = format!("{host}:443");
    let to = addr.to_socket_addrs().map_err(|e| format!("can't find {host} ({e})"))?.next().ok_or_else(|| format!("can't find {host}"))?;
    let stream = TcpStream::connect_timeout(&to, Duration::from_secs(15)).map_err(|e| format!("can't reach {host} ({e})"))?;
    stream.set_read_timeout(Some(Duration::from_secs(60))).map_err(|e| e.to_string())?;
    let session = tls::handshake(&stream, &addr)?;
    let out = Mutex::new(stream.try_clone().map_err(|e| e.to_string())?);
    tls::write(&session, &out, request).map_err(|e| format!("can't send to {host} ({e})"))?;
    let mut sock = stream;
    let mut buf = vec![0u8; 64 * 1024];
    let mut response = Vec::new();
    loop {
        match tls::read(&session, &mut sock, &out, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                response.extend_from_slice(buf.get(..n).unwrap_or(&[]));
                if response.len() > MAX {
                    return Err("the download is too large".into());
                }
            }
            // Servers often close without TLS's goodbye: what came is the answer (its length is
            // checked when it is read).
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof && !response.is_empty() => break,
            Err(e) => return Err(format!("the connection to {host} failed ({e})")),
        }
    }
    Ok(response)
}

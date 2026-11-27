//! Opening WebSockets to Discord: linking, the gateway and voice. They trust
//! the certificates the system trusts, as HTTPS requests do, so a CA added
//! for a reverse proxy works for both.

use std::sync::{Arc, OnceLock};

use rustls::ClientConfig;
use rustls_platform_verifier::ConfigVerifierExt;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, client_async_tls_with_config,
    connect_async_tls_with_config,
    tungstenite::{self, client::IntoClientRequest, error::TlsError, handshake::client::Response},
};

/// Opens a WebSocket to `request`: through the proxy and the encrypted DNS
/// Security set, when it set either.
pub(crate) async fn connect(
    request: impl IntoClientRequest + Unpin,
) -> tungstenite::Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response)> {
    let connector = Connector::Rustls(config().map_err(TlsError::Rustls)?);
    // Nagle's algorithm off, as browsers and Discord's client have it: a
    // small message (a heartbeat, a speaking flag) leaves at once, not held
    // until the server acknowledges the last, which Windows' delayed
    // acknowledgements can make wait up to 200 ms and showed as ping.
    if !crate::net::custom() {
        return connect_async_tls_with_config(request, None, true, Some(connector)).await;
    }
    let request = request.into_client_request()?;
    let host = request
        .uri()
        .host()
        .ok_or(tungstenite::Error::Url(
            tungstenite::error::UrlError::NoHostName,
        ))?
        .to_owned();
    let port = request
        .uri()
        .port_u16()
        .unwrap_or(match request.uri().scheme_str() {
            Some("ws") => 80,
            _ => 443,
        });
    let stream = crate::net::tcp(&host, port)
        .await
        .map_err(tungstenite::Error::Io)?;
    client_async_tls_with_config(request, stream, None, Some(connector)).await
}

/// The one TLS setup every socket shares: on Linux, making it reads and parses
/// the system's certificates. Without one, every reconnect would.
fn config() -> Result<Arc<ClientConfig>, rustls::Error> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    if let Some(config) = CONFIG.get() {
        return Ok(config.clone());
    }
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let config = Arc::new(ClientConfig::with_platform_verifier()?);
    Ok(CONFIG.get_or_init(|| config).clone())
}

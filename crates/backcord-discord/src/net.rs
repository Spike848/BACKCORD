//! How BACKCORD reaches Discord, as Settings' Security page sets it: through
//! a proxy (HTTP, HTTPS or SOCKS5, with a name and password if it asks),
//! and looking names up over encrypted DNS (DNS over HTTPS) instead of the
//! system's. HTTPS requests take both through reqwest; the WebSockets (the
//! gateway, voice's signalling) open their connection here. A call's sound
//! and video go over UDP, which proxies don't carry, straight to Discord's
//! voice servers. Set once as BACKCORD starts: a change takes effect on the
//! next start.

use std::{
    collections::HashMap,
    io,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex, OnceLock, RwLock},
    time::{Duration, Instant},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

/// The proxy's address, such as `socks5://127.0.0.1:1080`, and the
/// DNS-over-HTTPS server's (its JSON API), when set.
static PROXY: RwLock<Option<String>> = RwLock::new(None);
static DOH: RwLock<Option<String>> = RwLock::new(None);

/// Sets the proxy and the encrypted DNS server every connection from now on
/// goes through; none for straight connections and the system's lookups. A
/// proxy that isn't one [`valid_proxy`] takes is left out.
pub fn set(proxy: Option<String>, doh: Option<String>) {
    let proxy = proxy.filter(|proxy| valid_proxy(proxy));
    let doh = doh.filter(|doh| doh.starts_with("https://"));
    tracing::info!(
        proxy = proxy.is_some(),
        encrypted_dns = doh.is_some(),
        "network settings"
    );
    *PROXY
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = proxy;
    *DOH.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = doh;
}

fn proxy() -> Option<String> {
    PROXY
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn doh() -> Option<String> {
    DOH.read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Whether either is set, so connections go through here.
pub(crate) fn custom() -> bool {
    proxy().is_some() || doh().is_some()
}

/// Whether `address` is a proxy BACKCORD can use: `http://`, `https://`,
/// `socks5://` or `socks5h://`, a host and a port, perhaps a name and
/// password.
pub fn valid_proxy(address: &str) -> bool {
    url::Url::parse(address.trim()).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h")
            && url.host_str().is_some_and(|host| !host.is_empty())
            && url.port_or_known_default().is_some()
    })
}

/// `builder` with the proxy and the encrypted DNS, as set.
pub(crate) fn configure(mut builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    if let Some(address) = proxy() {
        match reqwest::Proxy::all(address.trim()) {
            Ok(proxy) => builder = builder.proxy(proxy),
            Err(error) => tracing::warn!(%error, "BACKCORD can't use that proxy"),
        }
    }
    if doh().is_some() {
        builder = builder.dns_resolver(Arc::new(Doh));
    }
    builder
}

/// A TCP connection to `host` at `port`: through the proxy when there's one,
/// else to the address the encrypted DNS or the system gives.
pub(crate) async fn tcp(host: &str, port: u16) -> io::Result<TcpStream> {
    if let Some(address) = proxy() {
        let url = url::Url::parse(address.trim())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let proxy_host = url.host_str().unwrap_or_default().to_owned();
        let proxy_port = url.port_or_known_default().unwrap_or(1080);
        let user = (!url.username().is_empty()).then(|| {
            let decode = |part: &str| percent_decode(part);
            (
                decode(url.username()),
                decode(url.password().unwrap_or_default()),
            )
        });
        let mut stream = connect_any(&proxy_host, proxy_port).await?;
        match url.scheme() {
            "socks5" | "socks5h" => socks5(&mut stream, host, port, user.as_ref()).await?,
            _ => http_connect(&mut stream, host, port, user.as_ref()).await?,
        }
        return Ok(stream);
    }
    connect_any(host, port).await
}

/// Connects to the first of `host`'s addresses that answers.
async fn connect_any(host: &str, port: u16) -> io::Result<TcpStream> {
    let mut last = io::Error::new(io::ErrorKind::NotFound, format!("no address for {host}"));
    for ip in resolve(host).await? {
        match TcpStream::connect(SocketAddr::new(ip, port)).await {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                return Ok(stream);
            }
            Err(error) => last = error,
        }
    }
    Err(last)
}

/// `host`'s addresses: itself if it's one, else from the encrypted DNS
/// (the system's should it fail), else the system's.
async fn resolve(host: &str) -> io::Result<Vec<IpAddr>> {
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        return Ok(vec![ip]);
    }
    if let Some(server) = doh() {
        match lookup(&server, host).await {
            Ok(found) if !found.is_empty() => return Ok(found),
            Ok(_) => tracing::debug!(host, "the encrypted DNS knows no address"),
            Err(error) => {
                tracing::warn!(error = format!("{error:#}"), host, "encrypted DNS failed");
            }
        }
    }
    Ok(tokio::net::lookup_host((host, 0))
        .await?
        .map(|address| address.ip())
        .collect())
}

/// Each host's addresses, and until when they hold.
type Kept = Mutex<Option<HashMap<String, (Vec<IpAddr>, Instant)>>>;

/// The addresses DNS over HTTPS gives for `host`, kept for as long as it
/// says they hold (a minute at least, an hour at most).
async fn lookup(server: &str, host: &str) -> anyhow::Result<Vec<IpAddr>> {
    static KEPT: Kept = Mutex::new(None);
    if let Some((found, until)) = KEPT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_or_insert_with(HashMap::new)
        .get(host)
        && *until > Instant::now()
    {
        return Ok(found.clone());
    }
    let mut found = Vec::new();
    let mut ttl = 3600u64;
    for kind in ["A", "AAAA"] {
        let mut url = url::Url::parse(server)?;
        url.query_pairs_mut()
            .append_pair("name", host)
            .append_pair("type", kind);
        let answer = doh_client()
            .get(url)
            .header("accept", "application/dns-json")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        let answer: serde_json::Value = serde_json::from_slice(&answer)?;
        for record in answer["Answer"].as_array().into_iter().flatten() {
            if let Some(ip) = record["data"]
                .as_str()
                .and_then(|data| data.parse::<IpAddr>().ok())
            {
                found.push(ip);
                ttl = ttl.min(record["TTL"].as_u64().unwrap_or(300));
            }
        }
        if !found.is_empty() {
            break;
        }
    }
    let until = Instant::now() + Duration::from_secs(ttl.clamp(60, 3600));
    KEPT.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_or_insert_with(HashMap::new)
        .insert(host.to_owned(), (found.clone(), until));
    Ok(found)
}

/// The client that asks the encrypted DNS: through the proxy, if any, and
/// the system's lookups for a server named rather than numbered.
fn doh_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let mut builder = reqwest::Client::builder().timeout(Duration::from_secs(8));
        if let Some(address) = proxy()
            && let Ok(proxy) = reqwest::Proxy::all(address.trim())
        {
            builder = builder.proxy(proxy);
        }
        builder.build().unwrap_or_default()
    })
}

/// reqwest's lookups, over the encrypted DNS.
struct Doh;

impl reqwest::dns::Resolve for Doh {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let found = resolve(&host).await?;
            let addresses: reqwest::dns::Addrs =
                Box::new(found.into_iter().map(|ip| SocketAddr::new(ip, 0)));
            Ok(addresses)
        })
    }
}

/// Asks a SOCKS5 proxy on `stream` for a connection to `host` at `port`,
/// with a name and password if given. The proxy looks the name up.
async fn socks5(
    stream: &mut TcpStream,
    host: &str,
    port: u16,
    user: Option<&(String, String)>,
) -> io::Result<()> {
    let refused = |what: &str| {
        io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("SOCKS5 proxy: {what}"),
        )
    };
    match user {
        Some(_) => stream.write_all(&[5, 2, 0, 2]).await?,
        None => stream.write_all(&[5, 1, 0]).await?,
    }
    let mut chosen = [0u8; 2];
    stream.read_exact(&mut chosen).await?;
    match (chosen[1], user) {
        (0, _) => {}
        (2, Some((name, password))) => {
            let mut hello = vec![1, name.len().min(255) as u8];
            hello.extend(name.as_bytes().iter().take(255));
            hello.push(password.len().min(255) as u8);
            hello.extend(password.as_bytes().iter().take(255));
            stream.write_all(&hello).await?;
            let mut answer = [0u8; 2];
            stream.read_exact(&mut answer).await?;
            if answer[1] != 0 {
                return Err(refused("wrong name or password"));
            }
        }
        _ => return Err(refused("no way to sign in it accepts")),
    }
    let mut ask = vec![5, 1, 0];
    match host.trim_matches(['[', ']']).parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            ask.push(1);
            ask.extend(ip.octets());
        }
        Ok(IpAddr::V6(ip)) => {
            ask.push(4);
            ask.extend(ip.octets());
        }
        Err(_) => {
            ask.push(3);
            ask.push(host.len().min(255) as u8);
            ask.extend(host.as_bytes().iter().take(255));
        }
    }
    ask.extend(port.to_be_bytes());
    stream.write_all(&ask).await?;
    let mut head = [0u8; 4];
    stream.read_exact(&mut head).await?;
    if head[1] != 0 {
        return Err(refused(&format!(
            "refused the connection (code {})",
            head[1]
        )));
    }
    let rest = match head[3] {
        1 => 4,
        4 => 16,
        3 => {
            let mut length = [0u8; 1];
            stream.read_exact(&mut length).await?;
            usize::from(length[0])
        }
        _ => return Err(refused("answered strangely")),
    };
    let mut bound = vec![0u8; rest + 2];
    stream.read_exact(&mut bound).await?;
    Ok(())
}

/// Asks an HTTP proxy on `stream` to CONNECT to `host` at `port`.
async fn http_connect(
    stream: &mut TcpStream,
    host: &str,
    port: u16,
    user: Option<&(String, String)>,
) -> io::Result<()> {
    use base64::Engine;
    use std::fmt::Write as _;
    let mut ask = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n");
    if let Some((name, password)) = user {
        let token = base64::engine::general_purpose::STANDARD.encode(format!("{name}:{password}"));
        let _ = write!(ask, "Proxy-Authorization: Basic {token}\r\n");
    }
    ask.push_str("\r\n");
    stream.write_all(ask.as_bytes()).await?;
    let mut answer = Vec::new();
    let mut byte = [0u8; 1];
    while !answer.ends_with(b"\r\n\r\n") {
        if answer.len() > 8192 || stream.read(&mut byte).await? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "HTTP proxy: no answer",
            ));
        }
        answer.push(byte[0]);
    }
    let line = String::from_utf8_lossy(&answer);
    let status = line.split_whitespace().nth(1).unwrap_or_default();
    if status != "200" {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("HTTP proxy refused the connection ({status})"),
        ));
    }
    Ok(())
}

/// `%XX` in a proxy's name or password, as its address writes it.
fn percent_decode(part: &str) -> String {
    let bytes = part.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Some(hex) = part.get(index + 1..index + 3)
            && let Ok(value) = u8::from_str_radix(hex, 16)
        {
            out.push(value);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxies_backcord_takes() {
        assert!(valid_proxy("socks5://127.0.0.1:1080"));
        assert!(valid_proxy("http://user:pass@proxy.example:8080"));
        assert!(valid_proxy("https://proxy.example"));
        assert!(!valid_proxy("ftp://proxy.example:21"));
        assert!(!valid_proxy("127.0.0.1:1080"));
        assert!(!valid_proxy(""));
    }

    #[test]
    fn a_password_with_escapes_reads_as_written() {
        assert_eq!(percent_decode("p%40ss%3Aword"), "p@ss:word");
        assert_eq!(percent_decode("plain"), "plain");
    }
}

//! Loopback HTTP and an authenticated bridge to the ordinary Unix protocol.
use anyhow::{Context, Result, bail};
use fastwebsockets::{Frame, OpCode, Payload, upgrade};
use http_body_util::Full;
use hyper::{
    Request, Response,
    body::{Bytes, Incoming},
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use std::{
    convert::Infallible,
    io::Read,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

#[derive(rust_embed::RustEmbed)]
#[folder = "../../web/dist/"]
// A Rust-only build still compiles; the page then 404s until `make web` has run.
#[allow_missing = true]
struct Assets;

struct State {
    origin: String,
    token: String,
    home: PathBuf,
}

pub async fn start(home: &Path) -> Result<()> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
    let port = listener.local_addr()?.port();
    let mut random = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
    let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let origin = format!("http://127.0.0.1:{port}");
    let info = serde_json::json!({"port":port,"pid":std::process::id(),"url":format!("{origin}/#{token}")});
    let mut file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(home.join("web.json"))?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    std::io::Write::write_all(&mut file, info.to_string().as_bytes())?;
    let state = Arc::new(State { origin, token, home: home.to_owned() });
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let state = state.clone();
            tokio::spawn(async move {
                let _ = hyper::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service_fn(move |req| handle(req, state.clone())))
                    .with_upgrades()
                    .await;
            });
        }
    });
    Ok(())
}

fn response(status: u16, body: impl Into<Bytes>) -> Response<Full<Bytes>> {
    let mut r = Response::new(Full::new(body.into()));
    *r.status_mut() = hyper::StatusCode::from_u16(status).expect("static HTTP status");
    r.headers_mut().insert("cache-control", "no-store".parse().expect("static header"));
    r.headers_mut().insert("x-content-type-options", "nosniff".parse().expect("static header"));
    r
}

fn same_origin<B>(req: &Request<B>, state: &State) -> bool {
    req.headers().get("host").and_then(|v| v.to_str().ok()) == state.origin.strip_prefix("http://")
        && req.headers().get("origin").and_then(|v| v.to_str().ok()) == Some(state.origin.as_str())
}

async fn handle(mut req: Request<Incoming>, state: Arc<State>) -> Result<Response<Full<Bytes>>, Infallible> {
    if req.headers().get("host").and_then(|v| v.to_str().ok()) != state.origin.strip_prefix("http://") {
        return Ok(response(403, "forbidden"));
    }
    if req.uri().path() == "/ws" {
        if !same_origin(&req, &state) {
            return Ok(response(403, "forbidden"));
        }
        return Ok(match upgrade::upgrade(&mut req) {
            Ok((r, fut)) => {
                tokio::spawn(async move {
                    let _ = bridge(fut, state).await;
                });
                r.map(|_| Full::new(Bytes::new()))
            }
            Err(_) => response(400, "invalid websocket upgrade"),
        });
    }
    if req.method() != hyper::Method::GET {
        return Ok(response(405, "method not allowed"));
    }
    let path = req.uri().path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let gzip = req.headers().get("accept-encoding").and_then(|v| v.to_str().ok()).is_some_and(|s| s.split(',').any(|v| v.trim() == "gzip"));
    let asset =
        if gzip { Assets::get(&format!("{path}.gz")).map(|a| (a, true)) } else { None }.or_else(|| Assets::get(path).map(|a| (a, false)));
    let Some((asset, compressed)) = asset else {
        return Ok(response(404, "not found"));
    };
    let mime = if path.ends_with(".js") {
        "text/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else {
        "application/octet-stream"
    };
    let mut r = response(200, asset.data.into_owned());
    r.headers_mut().insert("content-type", mime.parse().expect("static MIME"));
    r.headers_mut().insert("vary", "Accept-Encoding".parse().expect("static header"));
    r.headers_mut().insert(
        "content-security-policy",
        "default-src 'self'; connect-src 'self'; style-src 'self' 'unsafe-inline'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"
            .parse()
            .expect("static CSP"),
    );
    if compressed {
        r.headers_mut().insert("content-encoding", "gzip".parse().expect("static encoding"));
    }
    Ok(r)
}

async fn bridge(fut: upgrade::UpgradeFut, state: Arc<State>) -> Result<()> {
    let mut raw = fut.await?;
    raw.set_max_message_size(1024 * 1024);
    let (read, mut write) = raw.split(tokio::io::split);
    let mut read = fastwebsockets::FragmentCollectorRead::new(read);
    let (control_tx, mut control_rx) = tokio::sync::mpsc::channel(16);
    let first = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        read.read_frame(&mut |frame| {
            let tx = control_tx.clone();
            async move { tx.send(frame).await.map_err(|_| fastwebsockets::WebSocketError::ConnectionClosed) }
        }),
    )
    .await??;
    let value: serde_json::Value = serde_json::from_slice(&first.payload)?;
    if first.opcode != OpCode::Text || value["token"].as_str() != Some(&state.token) {
        bail!("unauthorized");
    }
    let req: protocol::Request = serde_json::from_value(value)?;
    if !matches!(req.op, protocol::Op::Hello { .. }) {
        bail!("hello required");
    }
    let stream = crate::client::connect_or_autostart(&state.home).await?;
    let (r, mut w) = stream.into_split();
    w.write_all(format!("{}\n", serde_json::to_string(&req)?).as_bytes()).await?;
    // Reads are not cancellation safe: give each direction its own future for its lifetime.
    let incoming = async move {
        loop {
            let tx = control_tx.clone();
            let frame = read
                .read_frame(&mut move |frame| {
                    let tx = tx.clone();
                    async move { tx.send(frame).await.map_err(|_| fastwebsockets::WebSocketError::ConnectionClosed) }
                })
                .await?;
            if frame.opcode == OpCode::Close {
                break;
            }
            if frame.opcode != OpCode::Text {
                continue;
            }
            let req: protocol::Request = serde_json::from_slice(&frame.payload)?;
            w.write_all(format!("{}\n", serde_json::to_string(&req)?).as_bytes()).await?;
        }
        anyhow::Ok(())
    };
    let outgoing = async move {
        let mut lines = BufReader::new(r).lines();
        loop {
            tokio::select! {
                control = control_rx.recv() => {
                    let Some(frame) = control else { break; };
                    write.write_frame(frame).await?;
                }
                line = lines.next_line() => {
                    let Some(line) = line? else { break; };
                    write.write_frame(Frame::text(Payload::Owned(line.into_bytes()))).await?;
                }
            }
        }
        anyhow::Ok(())
    };
    tokio::select! { result = incoming => result, result = outgoing => result }
}

struct Registration(PathBuf);
impl Drop for Registration {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub async fn run() -> Result<()> {
    let home = crate::paths::home()?;
    let lock = std::fs::File::create(home.join("web.lock"))?;
    if lock.try_lock().is_err() {
        let info: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(home.join("web.json"))?)?;
        bail!(
            "canopy web is already running at {} (pid {}). Stop it with Ctrl+C in that terminal.",
            info["url"].as_str().unwrap_or("unknown URL"),
            info["pid"]
        );
    }
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let _client = crate::client::Client::connect(&home).await?;
    let _registration = Registration(home.join("web.json"));
    start(&home).await?;
    let info: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(home.join("web.json"))?)?;
    let url = info["url"].as_str().context("missing web URL")?;
    println!("{url}");
    let command = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(command).arg(url).status();
    tokio::select! { _ = term.recv() => {}, _ = int.recv() => {} }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    async fn upgrade_request(port: u16, origin: &str) -> (tokio::net::TcpStream, String) {
        let mut tcp = tokio::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).await.unwrap();
        let request = format!(
            "GET /ws HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: {origin}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
        );
        tcp.write_all(request.as_bytes()).await.unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            header.push(tcp.read_u8().await.unwrap());
        }
        (tcp, String::from_utf8(header).unwrap())
    }

    #[tokio::test]
    async fn http_does_not_disclose_token_without_authentication() {
        let home = std::env::temp_dir().join(format!("canopy-http-{}", ulid::Ulid::generate()));
        std::fs::create_dir(&home).unwrap();
        start(&home).await.unwrap();
        let info: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(home.join("web.json")).unwrap()).unwrap();
        let port = info["port"].as_u64().unwrap() as u16;
        let token = info["url"].as_str().unwrap().split_once('#').unwrap().1;
        for (method, path, status) in [("POST", "/auth", "405"), ("GET", "/auth", "404"), ("GET", "/web.json", "404"), ("GET", "/", "200")]
        {
            let mut tcp = tokio::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).await.unwrap();
            let request = format!(
                "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
            );
            tcp.write_all(request.as_bytes()).await.unwrap();
            let mut response = String::new();
            tcp.read_to_string(&mut response).await.unwrap();
            assert!(response.starts_with(&format!("HTTP/1.1 {status}")), "{method} {path}: {response}");
            if path == "/" {
                assert!(response.contains("content-security-policy:"));
                assert!(response.contains("connect-src 'self'"));
                assert!(response.contains("frame-ancestors 'none'"));
            }
            assert!(!response.contains(token), "{method} {path} disclosed token");
        }
        std::fs::remove_dir_all(home).unwrap();
    }

    #[tokio::test]
    async fn websocket_rejects_cross_origin_and_authenticates_same_origin() {
        let home = std::env::temp_dir().join(format!("canopy-http-{}", ulid::Ulid::generate()));
        std::fs::create_dir(&home).unwrap();
        let unix = tokio::net::UnixListener::bind(home.join("sock")).unwrap();
        start(&home).await.unwrap();
        let info: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(home.join("web.json")).unwrap()).unwrap();
        let port = info["port"].as_u64().unwrap() as u16;
        let (_, response) = upgrade_request(port, "https://evil.example").await;
        assert!(response.starts_with("HTTP/1.1 403"));
        let origin = format!("http://127.0.0.1:{port}");
        let (tcp, _) = upgrade_request(port, &origin).await;
        let mut denied = fastwebsockets::WebSocket::after_handshake(tcp, fastwebsockets::Role::Client);
        denied.write_frame(Frame::text(Payload::Owned(br#"{"id":1,"op":"hello","version":1,"token":"wrong"}"#.to_vec()))).await.unwrap();
        assert!(tokio::time::timeout(std::time::Duration::from_secs(1), denied.read_frame()).await.unwrap().is_err());
        let (tcp, response) = upgrade_request(port, &origin).await;
        assert!(response.starts_with("HTTP/1.1 101"));
        let mut ws = fastwebsockets::WebSocket::after_handshake(tcp, fastwebsockets::Role::Client);
        let token = info["url"].as_str().unwrap().split('#').nth(1).unwrap();
        let hello = serde_json::json!({"id":1,"op":"hello","version":protocol::VERSION,"token":token}).to_string();
        ws.write_frame(Frame::text(Payload::Owned(hello.into_bytes()))).await.unwrap();
        let (stream, _) = tokio::time::timeout(std::time::Duration::from_secs(1), unix.accept()).await.unwrap().unwrap();
        let (r, mut w) = stream.into_split();
        let mut reader = BufReader::new(r);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        let request: protocol::Request = serde_json::from_str(&line).unwrap();
        assert!(matches!(request.op, protocol::Op::Hello { .. }));
        w.write_all(b"{\"t\":\"res\",\"id\":1,\"result\":{}}\n").await.unwrap();
        let frame = ws.read_frame().await.unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&frame.payload).unwrap()["id"], 1);
        drop(ws);
        drop(unix);
        std::fs::remove_dir_all(home).unwrap();
    }
}

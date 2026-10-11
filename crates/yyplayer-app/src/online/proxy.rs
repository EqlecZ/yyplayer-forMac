//! Local range proxy keeps CDN credentials out of mpv and enforces HTTPS redirects.
use super::{
    config::{Result, ensure},
    http::{self, Cancel},
    repository::{MAX_DOWNLOAD, Resolved},
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
pub struct Proxy {
    pub url: String,
    cancel: Arc<AtomicU64>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Proxy {
    pub fn start(audio: Resolved, timeout: u64, cache: PathBuf, key: String) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        let token = format!(
            "/{}",
            super::repository::hash(&format!(
                "{key}:{:?}:{}",
                std::time::SystemTime::now(),
                addr.port()
            ))
        );
        let url = format!("http://{addr}{token}");
        let cancel = Arc::new(AtomicU64::new(0));
        let generation = cancel.clone();
        let worker = std::thread::spawn(move || {
            let c = Cancel {
                generation,
                id: 0,
                permission: None,
            };
            let mut workers = vec![];
            while c.check().is_ok() {
                workers.retain(|w: &std::thread::JoinHandle<()>| !w.is_finished());
                match listener.accept() {
                    Ok((socket, _)) if workers.len() < 2 => {
                        let a = audio.clone();
                        let c = c.clone();
                        let token = token.clone();
                        let cache = cache.clone();
                        let key = key.clone();
                        workers.push(std::thread::spawn(move || {
                            let _ = serve(socket, &a, timeout, &token, cache, key, &c);
                        }));
                    }
                    Ok((mut socket, _)) => {
                        let _ = socket.write_all(
                            b"HTTP/1.1 503 Busy\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        );
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(20))
                    }
                    Err(_) => break,
                }
            }
            for w in workers {
                let _ = w.join();
            }
        });
        Ok(Self {
            url,
            cancel,
            worker: Some(worker),
        })
    }
    pub fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(|w| w.is_finished())
    }
    pub fn stop(&self) {
        self.cancel.store(1, Ordering::Release);
    }
    pub fn finish(mut self) {
        self.stop();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop();
    }
}
fn serve(
    mut socket: TcpStream,
    a: &Resolved,
    timeout: u64,
    token: &str,
    cache: PathBuf,
    key: String,
    c: &Cancel,
) -> Result<()> {
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let mut request = vec![];
    let mut b = [0; 1024];
    while !request.windows(4).any(|s| s == b"\r\n\r\n") {
        let n = socket.read(&mut b).map_err(|_| "本地流请求失败")?;
        ensure(n > 0 && request.len() + n <= 8192, "本地流请求无效")?;
        request.extend_from_slice(&b[..n]);
    }
    let request = std::str::from_utf8(&request).map_err(|_| "本地流请求无效")?;
    let mut lines = request.lines();
    let first = lines.next().unwrap_or_default();
    let parts: Vec<_> = first.split_whitespace().collect();
    if parts.len() != 3 || parts[1] != token || !["GET", "HEAD"].contains(&parts[0]) {
        let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
        return Err("本地流请求无效".into());
    }
    let range = lines.find_map(|l| {
        l.split_once(':')
            .filter(|(k, _)| k.eq_ignore_ascii_case("range"))
            .map(|(_, v)| v.trim())
    });
    if let Some(r) = range {
        ensure(
            r.starts_with("bytes=")
                && r[6..].bytes().all(|c| c.is_ascii_digit() || c == b'-')
                && r[6..].contains('-'),
            "范围请求无效",
        )?;
    }
    let mut response = match http::connect(&a.url, "GET", &a.headers, &[], timeout, range, c) {
        Ok(r) => r,
        Err(e) => {
            let _ = socket.write_all(
                b"HTTP/1.1 502 Upstream Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            return Err(e);
        }
    };
    let length = response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    ensure(length.is_none_or(|n| n <= MAX_DOWNLOAD), "媒体超过512MB")?;
    let mut header = format!(
        "HTTP/1.1 {} OK\r\nConnection: close\r\n",
        response.status().as_u16()
    );
    for name in [
        "content-length",
        "content-type",
        "content-range",
        "accept-ranges",
    ] {
        if let Some(v) = response.headers().get(name).and_then(|h| h.to_str().ok()) {
            header.push_str(&format!("{name}: {v}\r\n"));
        }
    }
    header.push_str("\r\n");
    socket
        .write_all(header.as_bytes())
        .map_err(|_| "播放端已关闭")?;
    if parts[0] == "HEAD" {
        return Ok(());
    }
    // Cache a complete sequential response only; seeks still pass Range to the CDN.
    let complete = response.status().as_u16() == 200
        || (range == Some("bytes=0-")
            && response
                .headers()
                .get("content-range")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.rsplit_once('/'))
                .and_then(|(_, s)| s.parse::<u64>().ok())
                == length);
    let managed = cache.join("yyplayer-stream");
    let partial = managed.join(format!(
        "{key}-{}.partial",
        socket.peer_addr().map(|a| a.port()).unwrap_or(0)
    ));
    let mut file = if complete {
        std::fs::create_dir_all(&managed).ok();
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&partial)
            .ok()
    } else {
        None
    };
    let owns_partial = file.is_some();
    let result = (|| {
        let mut reader = response.body_mut().as_reader();
        let mut buffer = [0; 65536];
        let mut total = 0;
        loop {
            c.check()?;
            let n = reader.read(&mut buffer).map_err(|_| "流读取失败")?;
            if n == 0 {
                break;
            }
            total += n as u64;
            ensure(total <= MAX_DOWNLOAD, "媒体超过512MB")?;
            socket.write_all(&buffer[..n]).map_err(|_| "播放端已关闭")?;
            if let Some(f) = file.as_mut()
                && f.write_all(&buffer[..n]).is_err()
            {
                file = None;
            }
        }
        ensure(
            total > 0 && length.is_none_or(|n| n == total),
            "媒体流不完整",
        )?;
        c.check()?;
        if let Some(f) = file.take() {
            let _ = f.sync_all();
            drop(f);
            let target = managed.join(format!("{key}.audio"));
            if !target.exists() {
                let _ = std::fs::rename(&partial, &target);
                super::cache::trim(&managed, "audio", 1024 * 1024 * 1024, &target);
            }
        }
        Ok(())
    })();
    drop(file);
    if owns_partial {
        let _ = std::fs::remove_file(partial);
    }
    result
}

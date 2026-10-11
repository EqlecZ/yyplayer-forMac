use super::config::{Result, Strings, ensure, secure_url};
use std::{
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use ureq::unversioned::transport::{Buffers, ConnectionDetails, Connector, NextTimeout, Transport};
// The pinned transport adapter supplies Android's per-read inactivity timeout,
// rather than a total-song deadline. Cancellation is observed every 250ms.
#[derive(Debug)]
struct IdleConnector {
    cancel: Cancel,
    timeout: u64,
}
#[derive(Debug)]
struct IdleTransport<T> {
    inner: T,
    cancel: Cancel,
    timeout: u64,
}
impl<T: Transport> Connector<T> for IdleConnector {
    type Out = IdleTransport<T>;
    fn connect(
        &self,
        _: &ConnectionDetails,
        chained: Option<T>,
    ) -> std::result::Result<Option<Self::Out>, ureq::Error> {
        Ok(chained.map(|inner| IdleTransport {
            inner,
            cancel: self.cancel.clone(),
            timeout: self.timeout,
        }))
    }
}
impl<T: Transport> Transport for IdleTransport<T> {
    fn buffers(&mut self) -> &mut dyn Buffers {
        self.inner.buffers()
    }
    fn transmit_output(
        &mut self,
        amount: usize,
        timeout: NextTimeout,
    ) -> std::result::Result<(), ureq::Error> {
        self.inner.transmit_output(amount, timeout)
    }
    fn await_input(&mut self, timeout: NextTimeout) -> std::result::Result<bool, ureq::Error> {
        let started = std::time::Instant::now();
        let budget = Duration::from_millis(self.timeout).min(*timeout.after);
        loop {
            if self.cancel.check().is_err() {
                return Err(ureq::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "cancelled",
                )));
            }
            let remaining = budget.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(ureq::Error::Timeout(timeout.reason));
            }
            let poll = NextTimeout {
                after: ureq::unversioned::transport::time::Duration::Exact(
                    remaining.min(Duration::from_millis(250)),
                ),
                reason: timeout.reason,
            };
            match self.inner.await_input(poll) {
                Err(ureq::Error::Timeout(_)) => continue,
                Err(ureq::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    continue;
                }
                result => return result,
            }
        }
    }
    fn is_open(&mut self) -> bool {
        self.inner.is_open()
    }
    fn is_tls(&self) -> bool {
        self.inner.is_tls()
    }
}
#[derive(Clone, Debug)]
pub struct Cancel {
    pub generation: Arc<AtomicU64>,
    pub id: u64,
    pub permission: Option<Arc<AtomicBool>>,
}
impl Cancel {
    pub fn check(&self) -> Result<()> {
        ensure(
            self.generation.load(Ordering::Acquire) == self.id
                && self
                    .permission
                    .as_ref()
                    .is_none_or(|p| p.load(Ordering::Acquire)),
            "操作已取消",
        )
    }
}
pub fn connect(
    url: &str,
    method: &str,
    headers: &Strings,
    body: &[u8],
    timeout: u64,
    range: Option<&str>,
    cancel: &Cancel,
) -> Result<ureq::http::Response<ureq::Body>> {
    let mut current = secure_url(url)?;
    let origin = current.clone();
    let roots = ureq::tls::RootCerts::PlatformVerifier;
    #[cfg(any(test, feature = "online-qa"))]
    let roots = if let Some(path) = std::env::var_os("YYPLAYER_ONLINE_TEST_CA") {
        ensure(
            matches!(origin.host_str(), Some("localhost" | "127.0.0.1")),
            "QA证书仅限本机",
        )?;
        let bytes = std::fs::read(path).map_err(|_| "QA证书不可读")?;
        let cert = ureq::tls::Certificate::from_pem(&bytes)
            .map_err(|_| "QA证书无效")?
            .to_owned();
        ureq::tls::RootCerts::new_with_certs(&[cert])
    } else {
        roots
    };
    let config = ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_resolve(Some(Duration::from_millis(timeout)))
        .timeout_send_request(Some(Duration::from_millis(timeout)))
        .timeout_send_body(Some(Duration::from_millis(timeout)))
        .timeout_connect(Some(Duration::from_millis(timeout)))
        .timeout_recv_response(Some(Duration::from_millis(timeout)))
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(roots).build())
        .build();
    use ureq::unversioned::transport::Connector;
    let connector =
        ureq::unversioned::transport::DefaultConnector::default().chain(IdleConnector {
            cancel: cancel.clone(),
            timeout,
        });
    let agent = ureq::Agent::with_parts(
        config,
        connector,
        ureq::unversioned::resolver::DefaultResolver::default(),
    );
    for _ in 0..6 {
        cancel.check()?;
        let mut req = ureq::http::Request::builder()
            .method(method)
            .uri(current.as_str())
            .header("Accept-Encoding", "identity");
        if current.host_str() == origin.host_str()
            && current.port_or_known_default() == origin.port_or_known_default()
        {
            for (k, v) in headers {
                req = req.header(k, v);
            }
        }
        if let Some(r) = range {
            req = req.header("Range", r);
        }
        let request = req.body(body).map_err(|_| "访问头或请求无效")?;
        // Do not include ureq's URL-bearing error in UI/logs (query may contain credentials).
        let response = agent.run(request).map_err(|error| {
            let _ = error;
            "HTTPS请求失败（网络、证书、超时或HTTP错误）"
        })?;
        let status = response.status().as_u16();
        if [301, 302, 303, 307, 308].contains(&status) {
            ensure(method == "GET", "POST重定向不受支持")?;
            let location = response
                .headers()
                .get("location")
                .and_then(|s| s.to_str().ok())
                .ok_or("重定向缺少地址")?;
            current = secure_url(
                current
                    .join(location)
                    .map_err(|_| "重定向地址无效")?
                    .as_str(),
            )?;
        } else {
            ensure(
                (200..300).contains(&status),
                &format!("服务返回HTTP {status}"),
            )?;
            cancel.check()?;
            return Ok(response);
        }
    }
    Err("重定向次数过多".into())
}
pub fn read(
    mut response: ureq::http::Response<ureq::Body>,
    limit: usize,
    cancel: &Cancel,
) -> Result<Vec<u8>> {
    if let Some(n) = response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
    {
        ensure(n <= limit, "响应超过大小限制")?;
    }
    let mut out = vec![];
    let mut buffer = [0; 8192];
    let mut reader = response.body_mut().as_reader();
    loop {
        cancel.check()?;
        let n = reader.read(&mut buffer).map_err(|_| "响应读取失败")?;
        if n == 0 {
            break;
        }
        ensure(out.len() + n <= limit, "响应超过大小限制")?;
        out.extend_from_slice(&buffer[..n]);
    }
    cancel.check()?;
    Ok(out)
}

//! Android ApiConfig v1 contract; declarative templates, never scripts.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub type Strings = BTreeMap<String, String>;
pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub version: u32,
    pub providers: Vec<Provider>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub headers: Strings,
    #[serde(default)]
    pub media_headers: BTreeMap<String, Strings>,
    pub sources: indexmap::IndexMap<String, String>,
    #[serde(default)]
    pub qualities: Vec<Quality>,
    pub search: Option<Endpoint>,
    pub stream: Option<Endpoint>,
    pub download: Option<Endpoint>,
    pub lyric_search: Option<Endpoint>,
    pub lyric: Option<Endpoint>,
    pub artwork: Option<Endpoint>,
    #[serde(default = "timeout")]
    pub timeout_ms: u64,
    #[serde(default = "requests")]
    pub requests_per_window: usize,
    #[serde(default = "window")]
    pub window_seconds: u64,
    #[serde(default = "page_size")]
    pub page_size: usize,
    #[serde(default)]
    pub variables: Strings,
    #[serde(default)]
    pub track_fields: Strings,
    #[serde(default)]
    pub api_hosts: BTreeSet<String>,
}
fn timeout() -> u64 {
    15000
}
fn requests() -> usize {
    40
}
fn window() -> u64 {
    300
}
fn page_size() -> usize {
    20
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Quality {
    pub id: String,
    pub name: String,
    pub value: String,
    #[serde(default = "extension")]
    pub extension: String,
    #[serde(default)]
    pub parameters: Strings,
    #[serde(default)]
    pub variables: Strings,
    #[serde(default)]
    pub sources: BTreeSet<String>,
}
fn extension() -> String {
    "mp3".into()
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Endpoint {
    pub url: Option<String>,
    #[serde(default = "method")]
    pub method: String,
    #[serde(default = "encoding")]
    pub encoding: String,
    #[serde(default)]
    pub parameters: Strings,
    #[serde(default)]
    pub headers: Strings,
    pub body: Option<Value>,
    pub response: Value,
}
fn method() -> String {
    "GET".into()
}
fn encoding() -> String {
    "query".into()
}
pub fn secure_url(s: &str) -> Result<url::Url> {
    let u = url::Url::parse(s).map_err(|_| "需要有效 HTTPS 地址")?;
    ensure(
        u.scheme() == "https"
            && u.host_str().is_some()
            && u.username().is_empty()
            && u.password().is_none()
            && u.fragment().is_none()
            && s.len() <= 8192,
        "需要有效 HTTPS 地址",
    )?;
    Ok(u)
}
pub fn ensure(ok: bool, msg: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(msg.into()) }
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 48
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
fn host(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
}
fn string_map(m: &Strings) -> Result<()> {
    ensure(
        m.len() <= 64 && m.iter().all(|(k, v)| k.len() <= 128 && v.len() <= 8192),
        "字符串字典超出范围",
    )
}
pub fn headers(m: &Strings) -> Result<()> {
    string_map(m)?;
    for (k, v) in m {
        ensure(
            !k.is_empty()
                && k.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+.^_`|~-".contains(&c))
                && !v.contains(['\r', '\n', '\0']),
            "非法访问头",
        )?;
        ensure(
            ![
                "host",
                "content-length",
                "connection",
                "transfer-encoding",
                "range",
            ]
            .contains(&k.to_ascii_lowercase().as_str()),
            "访问头由客户端管理",
        )?;
    }
    Ok(())
}
pub fn depth(text: &str) -> Result<()> {
    let (mut n, mut quoted, mut escaped) = (0, false, false);
    for c in text.chars() {
        if quoted {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
        } else {
            match c {
                '"' => quoted = true,
                '{' | '[' => {
                    n += 1;
                    ensure(n <= 32, "JSON 嵌套过深")?;
                }
                '}' | ']' => n -= 1,
                _ => {}
            }
        }
    }
    Ok(())
}
pub fn template(s: &str, vars: &Strings, encode: bool) -> Result<String> {
    let mut out = String::new();
    let mut tail = s;
    while let Some(pos) = tail.find('{') {
        out.push_str(&tail[..pos]);
        tail = &tail[pos..];
        if let Some(end) = tail.find('}') {
            let name = &tail[1..end];
            if !name.is_empty()
                && name.as_bytes()[0].is_ascii_alphabetic()
                && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
            {
                let value = vars
                    .get(name)
                    .ok_or_else(|| format!("配置使用了未知变量 {name}"))?;
                if encode {
                    out.push_str(
                        &url::form_urlencoded::byte_serialize(value.as_bytes()).collect::<String>(),
                    );
                } else {
                    out.push_str(value);
                }
                tail = &tail[end + 1..];
                continue;
            }
        }
        out.push('{');
        tail = &tail[1..];
    }
    out.push_str(tail);
    Ok(out)
}
pub fn body(v: &Value, vars: &Strings) -> Result<Value> {
    Ok(match v {
        Value::String(s) => Value::String(template(s, vars, false)?),
        Value::Array(a) => Value::Array(a.iter().map(|v| body(v, vars)).collect::<Result<_>>()?),
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, v)| Ok((k.clone(), body(v, vars)?)))
                .collect::<Result<_>>()?,
        ),
        _ => v.clone(),
    })
}
pub fn field<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(v);
    }
    if path.len() > 256 {
        return None;
    }
    path.split('.').try_fold(v, |n, k| {
        if n.is_array() {
            n.get(k.parse::<usize>().ok()?)
        } else {
            n.get(k)
        }
    })
}
pub fn text(v: &Value, path: &str) -> String {
    fn display(v: &Value) -> String {
        match v {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            Value::Array(a) => a.iter().map(display).collect::<Vec<_>>().join(" / "),
            _ => v.to_string(),
        }
    }
    field(v, path).map(display).unwrap_or_default()
}
pub fn response(e: &Endpoint, key: &str, default: &str) -> String {
    e.response
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .into()
}
impl Config {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure(bytes.len() <= 256 * 1024, "配置超过 256KB")?;
        let s = std::str::from_utf8(bytes)
            .map_err(|_| "配置不是 UTF-8")?
            .trim_start_matches('\u{feff}');
        depth(s)?;
        let mut c: Self = serde_json::from_str(s).map_err(|e| format!("JSON 配置无效：{e}"))?;
        ensure(
            c.version == 1 && (1..=16).contains(&c.providers.len()),
            "需要版本1和1–16个API",
        )?;
        let mut ids = BTreeSet::new();
        for p in &mut c.providers {
            ensure(
                identifier(&p.id) && ids.insert(p.id.clone()),
                "API id非法或重复",
            )?;
            ensure(
                !p.name.is_empty() && p.name.chars().count() <= 80,
                "API名称过长",
            )?;
            let base = secure_url(&p.base_url)?;
            p.api_hosts.insert(base.host_str().unwrap().into());
            ensure(
                p.api_hosts.len() <= 16 && p.api_hosts.iter().all(|h| host(h)),
                "非法API主机",
            )?;
            ensure(
                p.sources
                    .iter()
                    .all(|(k, v)| k.len() <= 128 && v.len() <= 8192),
                "来源名称超出范围",
            )?;
            ensure((1..=16).contains(&p.sources.len()), "需要音乐来源")?;
            ensure(
                (1000..=60000).contains(&p.timeout_ms)
                    && (1..=1000).contains(&p.requests_per_window)
                    && (1..=3600).contains(&p.window_seconds)
                    && (1..=200).contains(&p.page_size),
                "限流、超时或分页超出范围",
            )?;
            string_map(&p.variables)?;
            string_map(&p.track_fields)?;
            let mut vars: Strings = [
                "query", "source", "page", "limit", "id", "lyricId", "picId", "title", "artist",
                "album", "quality",
            ]
            .into_iter()
            .map(|s| (s.into(), "test".into()))
            .collect();
            for (k, v) in p.variables.iter().chain(p.track_fields.iter()) {
                ensure(
                    !vars.contains_key(k)
                        && k.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                        && k.len() <= 48
                        && k.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'),
                    "非法或重复变量",
                )?;
                vars.insert(k.clone(), v.clone());
            }
            headers(&p.headers)?;
            for v in p.headers.values() {
                template(v, &vars, false)?;
            }
            for (h, m) in &p.media_headers {
                ensure(host(h), "媒体头需要精确主机")?;
                headers(m)?;
            }
            let mut qids = BTreeSet::new();
            ensure(p.qualities.len() <= 12, "音质选项过多")?;
            for q in &p.qualities {
                ensure(
                    identifier(&q.id)
                        && qids.insert(&q.id)
                        && !q.value.is_empty()
                        && q.value.len() <= 256
                        && !q.name.is_empty()
                        && q.name.chars().count() <= 80,
                    "非法音质",
                )?;
                ensure(
                    [
                        "mp3", "flac", "m4a", "ogg", "opus", "wav", "aac", "aiff", "ape",
                    ]
                    .contains(&q.extension.as_str())
                        && q.sources.iter().all(|s| p.sources.contains_key(s)),
                    "非法音质格式或来源",
                )?;
                string_map(&q.parameters)?;
                string_map(&q.variables)?;
                ensure(
                    q.variables.keys().all(|k| p.variables.contains_key(k)),
                    "音质变量未声明",
                )?;
                for v in q.parameters.values() {
                    template(v, &vars, false)?;
                }
            }
            for (name, e) in [
                ("search", &p.search),
                ("stream", &p.stream),
                ("download", &p.download),
                ("lyricSearch", &p.lyric_search),
                ("lyric", &p.lyric),
                ("artwork", &p.artwork),
            ] {
                let Some(e) = e else {
                    continue;
                };
                p.check_endpoint(&secure_url(&template(
                    e.url.as_deref().unwrap_or(&p.base_url),
                    &vars,
                    true,
                )?)?)?;
                ensure(
                    ["GET", "POST"].contains(&e.method.as_str())
                        && ["query", "form", "json"].contains(&e.encoding.as_str())
                        && (e.method != "GET" || e.encoding == "query"),
                    "非法方法或编码",
                )?;
                ensure(
                    e.response.is_object()
                        && ["json", "text"].contains(&response(e, "mode", "json").as_str())
                        && ["seconds", "milliseconds"]
                            .contains(&response(e, "durationUnit", "seconds").as_str())
                        && ["bytes", "KB"].contains(&response(e, "sizeUnit", "bytes").as_str()),
                    "非法响应映射",
                )?;
                if ["search", "lyricSearch"].contains(&name) {
                    ensure(
                        response(e, "mode", "json") == "json"
                            && e.response.get("items").is_some_and(Value::is_string),
                        "搜索必须配置JSON条目路径",
                    )?;
                    let fields: Strings = serde_json::from_value(
                        e.response.get("fields").cloned().unwrap_or_default(),
                    )
                    .map_err(|_| "搜索字段无效")?;
                    string_map(&fields)?;
                    ensure(
                        fields.contains_key("id") && fields.contains_key("title"),
                        "搜索缺少id/title",
                    )?;
                }
                string_map(&e.parameters)?;
                headers(&e.headers)?;
                for v in e.parameters.values().chain(e.headers.values()) {
                    template(v, &vars, false)?;
                }
                if let Some(b) = &e.body {
                    ensure(e.encoding == "json" && b.is_object(), "body需要JSON对象")?;
                    body(b, &vars)?;
                }
            }
            ensure(
                (p.search.is_some() && p.stream.is_some() && !p.qualities.is_empty())
                    || (p.lyric_search.is_some() && p.lyric.is_some()),
                "缺少音乐或歌词操作",
            )?;
        }
        Ok(c)
    }
}
impl Provider {
    pub fn check_endpoint(&self, u: &url::Url) -> Result<()> {
        let base = secure_url(&self.base_url)?;
        ensure(
            self.api_hosts.contains(u.host_str().unwrap_or_default())
                && (u.port().is_none() || u.port() == base.port()),
            "操作地址未列入apiHosts",
        )
    }
}

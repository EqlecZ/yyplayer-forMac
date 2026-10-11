use super::{
    config::*,
    http::{self, Cancel},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
pub const MAX_DOWNLOAD: u64 = 512 * 1024 * 1024;
#[derive(Clone, Debug, Default)]
pub struct Song {
    pub provider: String,
    pub source: String,
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub lyric_id: String,
    pub pic_id: String,
    pub artwork_url: String,
    pub duration_ms: Option<u64>,
    pub qualities: Option<BTreeSet<String>>,
    pub variables: Strings,
}
impl Song {
    pub fn key(&self) -> String {
        format!("{}:{}:{}", self.provider, self.source, self.id)
    }
}
pub fn media_key(p: &Provider, s: &Song, q: &Quality) -> Result<String> {
    let signature = serde_json::to_string(p).map_err(|_| "配置序列化失败")?
        + &serde_json::to_string(&s.variables).map_err(|_| "曲目变量无效")?;
    Ok(hash(&format!("{signature}:{}:{}", s.key(), q.id)))
}
#[derive(Clone)]
pub struct Resolved {
    pub url: String,
    pub report: String,
    pub headers: Strings,
    pub reported_size: Option<u64>,
    pub reported_format: String,
}
impl Resolved {
    pub fn details(&self) -> String {
        let mut parts = vec![];
        if !self.reported_format.is_empty() {
            parts.push(format!("服务格式：{}", self.reported_format));
        }
        if let Some(size) = self.reported_size {
            parts.push(format!("服务大小：{:.1} MB", size as f64 / 1048576.0));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("；{}", parts.join("；"))
        }
    }
}
#[derive(Default)]
struct Shared {
    times: BTreeMap<String, VecDeque<Instant>>,
    resolved: BTreeMap<String, (Instant, Resolved)>,
}
#[derive(Clone, Default)]
pub struct Repository {
    shared: Arc<Mutex<Shared>>,
}
pub fn variables(
    p: &Provider,
    song: Option<&Song>,
    q: Option<&Quality>,
    query: &str,
    source: &str,
    page: usize,
) -> Strings {
    let empty = Song::default();
    let s = song.unwrap_or(&empty);
    let mut v = p.variables.clone();
    if let Some(q) = q {
        v.extend(q.variables.clone());
    }
    v.extend(s.variables.clone());
    v.extend(
        [
            ("query", query.into()),
            ("source", source.into()),
            ("page", page.to_string()),
            ("limit", p.page_size.to_string()),
            ("id", s.id.clone()),
            ("lyricId", s.lyric_id.clone()),
            ("picId", s.pic_id.clone()),
            ("title", s.title.clone()),
            ("artist", s.artist.clone()),
            ("album", s.album.clone()),
            ("quality", q.map(|q| q.value.clone()).unwrap_or_default()),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v)),
    );
    v
}
impl Repository {
    pub fn clear_resolved(&self) {
        self.shared.lock().unwrap().resolved.clear();
    }
    pub fn call(
        &self,
        p: &Provider,
        e: &Endpoint,
        vars: &Strings,
        overrides: &Strings,
        cancel: &Cancel,
    ) -> Result<Value> {
        cancel.check()?;
        let mut u = secure_url(&template(
            e.url.as_deref().unwrap_or(&p.base_url),
            vars,
            true,
        )?)?;
        p.check_endpoint(&u)?;
        let mut params = e.parameters.clone();
        params.extend(overrides.clone());
        let params: Strings = params
            .iter()
            .map(|(k, v)| Ok((k.clone(), template(v, vars, false)?)))
            .collect::<Result<_>>()?;
        let mut h: Strings = p
            .headers
            .iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
            .collect();
        h.extend(
            e.headers
                .iter()
                .map(|(k, v)| (k.to_ascii_lowercase(), v.clone())),
        );
        let mut h: Strings = h
            .iter()
            .map(|(k, v)| Ok((k.clone(), template(v, vars, false)?)))
            .collect::<Result<_>>()?;
        headers(&h)?;
        let mut data = vec![];
        match e.encoding.as_str() {
            "query" => {
                if !params.is_empty() {
                    u.query_pairs_mut().extend_pairs(params.iter());
                }
            }
            "form" => {
                data = url::form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(params.iter())
                    .finish()
                    .into_bytes();
                h.insert(
                    "content-type".into(),
                    "application/x-www-form-urlencoded; charset=UTF-8".into(),
                );
            }
            _ => {
                data = serde_json::to_vec(&match &e.body {
                    Some(b) => body(b, vars)?,
                    None => serde_json::to_value(&params).unwrap(),
                })
                .map_err(|_| "JSON编码失败")?;
                h.insert(
                    "content-type".into(),
                    "application/json; charset=UTF-8".into(),
                );
            }
        }
        {
            let mut shared = self.shared.lock().unwrap();
            let times = shared.times.entry(p.id.clone()).or_default();
            while times
                .front()
                .is_some_and(|t| t.elapsed() >= Duration::from_secs(p.window_seconds))
            {
                times.pop_front();
            }
            ensure(
                times.len() < p.requests_per_window,
                "请求较频繁，请稍后再试",
            )?;
            times.push_back(Instant::now());
        }
        let bytes = http::read(
            http::connect(u.as_str(), &e.method, &h, &data, p.timeout_ms, None, cancel)?,
            2 * 1024 * 1024,
            cancel,
        )?;
        let s = std::str::from_utf8(&bytes)
            .map_err(|_| "响应不是UTF-8")?
            .trim_start_matches('\u{feff}');
        ensure(
            !s.trim_start().starts_with('<'),
            "服务返回网页验证，当前API不可用",
        )?;
        if response(e, "mode", "json") == "text" {
            Ok(Value::String(s.into()))
        } else {
            depth(s)?;
            serde_json::from_str(s).map_err(|_| "服务返回无效JSON".into())
        }
    }
    pub fn search(
        &self,
        p: &Provider,
        source: &str,
        query: &str,
        page: usize,
        lyrics: bool,
        cancel: &Cancel,
    ) -> Result<Vec<Song>> {
        ensure(
            p.sources.contains_key(source)
                && !query.trim().is_empty()
                && query.chars().count() <= 200
                && page >= 1,
            "关键词或来源无效",
        )?;
        let e = (if lyrics { &p.lyric_search } else { &p.search })
            .as_ref()
            .ok_or("未配置搜索接口")?;
        let v = self.call(
            p,
            e,
            &variables(p, None, None, query, source, page),
            &Strings::new(),
            cancel,
        )?;
        let rows = field(&v, &response(e, "items", ""))
            .and_then(Value::as_array)
            .ok_or("服务未返回搜索列表")?;
        ensure(rows.len() <= 200, "单次搜索超过200条")?;
        let mut seen = BTreeSet::new();
        let mut songs = vec![];
        for row in rows {
            let f = |name: &str| {
                text(row, e.response["fields"][name].as_str().unwrap_or(name))
                    .chars()
                    .take(512)
                    .collect::<String>()
            };
            let (id, title) = (f("id"), f("title"));
            if id.is_empty() || title.is_empty() {
                continue;
            }
            let src = f("source");
            let src = if src.is_empty() { source.into() } else { src };
            if !seen.insert((src.clone(), id.clone())) {
                continue;
            }
            let lyric_id = f("lyricId");
            let duration = f("duration")
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && *n >= 0.0 && *n <= 86400000.0)
                .map(|n| {
                    (n * if response(e, "durationUnit", "seconds") == "milliseconds" {
                        1.0
                    } else {
                        1000.0
                    }) as u64
                });
            let qs = e.response["fields"]["qualities"]
                .as_str()
                .and_then(|path| field(row, path));
            let qualities = qs.and_then(|v| match v {
                Value::Array(a) => Some(a.iter().map(|v| text(v, "")).collect()),
                Value::String(s) => Some(s.split(',').map(|s| s.trim().into()).collect()),
                _ => None,
            });
            songs.push(Song {
                provider: p.id.clone(),
                source: src,
                id: id.clone(),
                title,
                artist: f("artist"),
                album: f("album"),
                lyric_id: if lyric_id.is_empty() { id } else { lyric_id },
                pic_id: f("picId"),
                artwork_url: f("artworkUrl"),
                duration_ms: duration,
                qualities,
                variables: p
                    .track_fields
                    .iter()
                    .map(|(k, path)| (k.clone(), text(row, path).chars().take(8192).collect()))
                    .collect(),
            });
        }
        if lyrics {
            songs.sort_by_key(|s| {
                (
                    !s.title.trim().eq_ignore_ascii_case(query.trim()),
                    s.title.clone(),
                )
            });
        }
        Ok(songs)
    }
    pub fn resolve(
        &self,
        p: &Provider,
        s: &Song,
        q: &Quality,
        download: bool,
        cancel: &Cancel,
    ) -> Result<Resolved> {
        ensure(eligible(s, q), "此歌曲或来源不支持该音质")?;
        let key = format!("{}:{download}", media_key(p, s, q)?);
        if let Some((t, r)) = self.shared.lock().unwrap().resolved.get(&key)
            && t.elapsed() < Duration::from_secs(60)
        {
            cancel.check()?;
            return Ok(r.clone());
        }
        let e = (if download {
            p.download.as_ref().or(p.stream.as_ref())
        } else {
            p.stream.as_ref()
        })
        .ok_or("未配置播放地址")?;
        let v = self.call(
            p,
            e,
            &variables(p, Some(s), Some(q), "", &s.source, 1),
            &q.parameters,
            cancel,
        )?;
        let url = text(&v, &response(e, "url", "url"));
        let u = secure_url(&url)?;
        let report: String = text(&v, &response(e, "quality", "br"))
            .chars()
            .take(512)
            .collect();
        ensure(
            !p.qualities
                .iter()
                .any(|other| (other.id == report || other.value == report) && other.id != q.id),
            "服务实际返回其他音质，请选择对应档位",
        )?;
        let r = Resolved {
            url,
            report,
            reported_format: text(&v, &response(e, "format", "format"))
                .chars()
                .take(512)
                .collect(),
            reported_size: text(&v, &response(e, "size", "size"))
                .parse::<f64>()
                .ok()
                .map(|n| {
                    n * if response(e, "sizeUnit", "bytes") == "KB" {
                        1024.0
                    } else {
                        1.0
                    }
                })
                .filter(|n| n.is_finite() && *n > 0.0 && *n <= MAX_DOWNLOAD as f64)
                .map(|n| n as u64),
            headers: p
                .media_headers
                .get(u.host_str().unwrap())
                .cloned()
                .unwrap_or_default(),
        };
        let mut shared = self.shared.lock().unwrap();
        if shared.resolved.len() >= 128 {
            shared.resolved.clear();
        }
        shared.resolved.insert(key, (Instant::now(), r.clone()));
        Ok(r)
    }
    pub fn lyric(
        &self,
        p: &Provider,
        s: &Song,
        directory: &Path,
        cancel: &Cancel,
    ) -> Result<String> {
        cancel.check()?;
        let selected = directory
            .join("selected")
            .join(format!("{}.lrc", hash(&s.key())));
        let target = if selected.is_file() {
            selected
        } else {
            directory.join(format!("{}.lrc", hash(&s.key())))
        };
        if target.is_file() {
            let text = read_file(&target, 1_000_000)?;
            player_core::lyrics::Lyrics::parse(&String::from_utf8_lossy(&text))?;
            return String::from_utf8(text).map_err(|_| "缓存歌词编码无效".into());
        }
        let e = p.lyric.as_ref().ok_or("未配置歌词接口")?;
        let v = self.call(
            p,
            e,
            &variables(p, Some(s), None, "", &s.source, 1),
            &Strings::new(),
            cancel,
        )?;
        let original = text(&v, &response(e, "lyric", "lyric"));
        ensure(!original.trim().is_empty(), "候选暂无歌词")?;
        let translation = text(&v, &response(e, "translation", "__missing__"));
        let text = merge_lyrics(&original, &translation)?;
        cancel.check()?;
        crate::services::atomic_bytes(&target, text.as_bytes())?;
        super::cache::trim(directory, "lrc", 16 * 1024 * 1024, &target);
        Ok(text)
    }
    pub fn artwork(
        &self,
        p: &Provider,
        s: &Song,
        directory: &Path,
        cancel: &Cancel,
    ) -> Result<Option<(u32, u32, Vec<u8>)>> {
        cancel.check()?;
        let target = directory.join(format!(
            "{}.jpg",
            hash(&format!("{}:{}", s.key(), s.pic_id))
        ));
        let bytes = if target.is_file() {
            read_file(&target, 8 * 1024 * 1024)?
        } else {
            let url = if !s.artwork_url.is_empty() {
                s.artwork_url.clone()
            } else if let Some(e) = &p.artwork {
                if s.pic_id.is_empty() {
                    return Ok(None);
                }
                let v = self.call(
                    p,
                    e,
                    &variables(p, Some(s), None, "", &s.source, 1),
                    &Strings::new(),
                    cancel,
                )?;
                text(&v, &response(e, "url", "url"))
            } else {
                return Ok(None);
            };
            if url.is_empty() {
                return Ok(None);
            }
            let u = secure_url(&url)?;
            let h = p
                .media_headers
                .get(u.host_str().unwrap())
                .cloned()
                .unwrap_or_default();
            http::read(
                http::connect(&url, "GET", &h, &[], p.timeout_ms, None, cancel)?,
                8 * 1024 * 1024,
                cancel,
            )?
        };
        let mut reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
            .with_guessed_format()
            .map_err(|_| "封面格式无效")?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode().map_err(|_| "封面无法解码或超出尺寸")?;
        let image = image.thumbnail(image.width().min(1024), image.height().min(1024));
        cancel.check()?;
        if !target.is_file() {
            let mut jpg = std::io::Cursor::new(vec![]);
            image
                .write_to(&mut jpg, image::ImageFormat::Jpeg)
                .map_err(|_| "封面保存失败")?;
            crate::services::atomic_bytes(&target, &jpg.into_inner())?;
            super::cache::trim(directory, "jpg", 128 * 1024 * 1024, &target);
        }
        let image = image.into_rgba8();
        Ok(Some((image.width(), image.height(), image.into_raw())))
    }
}
pub fn eligible(s: &Song, q: &Quality) -> bool {
    (q.sources.is_empty() || q.sources.contains(&s.source))
        && s.qualities
            .as_ref()
            .is_none_or(|qs| qs.contains(&q.id) || qs.contains(&q.value))
}
pub fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
pub fn read_file(p: &Path, limit: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = vec![];
    std::fs::File::open(p)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    ensure(bytes.len() as u64 <= limit, "文件超过大小限制")?;
    Ok(bytes)
}
pub fn merge_lyrics(a: &str, b: &str) -> Result<String> {
    let a_parsed = player_core::lyrics::Lyrics::parse(a)?;
    if b.trim().is_empty() {
        return Ok(a.into());
    }
    let b_parsed = player_core::lyrics::Lyrics::parse(b)?;
    if a_parsed.lines.is_empty() {
        let s = format!("{a}\n\n{b}");
        player_core::lyrics::Lyrics::parse(&s)?;
        return Ok(s);
    }
    let mut rows = BTreeMap::<i64, Vec<String>>::new();
    for l in &a_parsed.lines {
        rows.entry(l.milliseconds).or_default().push(l.text.clone());
    }
    for l in b_parsed.lines {
        if let Some(row) = rows.get_mut(&l.milliseconds)
            && !row.contains(&l.text)
        {
            row.push(l.text);
        }
    }
    let s = rows
        .into_iter()
        .map(|(ms, row)| {
            format!(
                "[{:02}:{:02}.{:03}]{}",
                ms / 60000,
                ms / 1000 % 60,
                ms % 1000,
                row.join(" / ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    player_core::lyrics::Lyrics::parse(&s)?;
    Ok(s)
}
pub fn audio_extension(bytes: &[u8]) -> Option<&'static str> {
    let at = |n: usize, s: &[u8]| bytes.get(n..n + s.len()) == Some(s);
    let byte = |n: usize| bytes.get(n).copied().unwrap_or(0);
    let mut offset = 0;
    if at(0, b"ID3") && bytes.len() >= 10 {
        if (6..10).any(|i| byte(i) >= 128) {
            return None;
        }
        offset = 10
            + ((byte(6) as usize) << 21)
            + ((byte(7) as usize) << 14)
            + ((byte(8) as usize) << 7)
            + byte(9) as usize;
        if byte(5) & 16 != 0 {
            offset += 10;
        }
    }
    if at(offset, b"fLaC") {
        Some("flac")
    } else if (at(0, b"RIFF") || at(0, b"RF64")) && at(8, b"WAVE") {
        Some("wav")
    } else if at(4, b"ftyp") {
        Some("m4a")
    } else if at(0, b"OggS") {
        Some(if bytes.windows(8).any(|s| s == b"OpusHead") {
            "opus"
        } else {
            "ogg"
        })
    } else if at(0, b"FORM") && (at(8, b"AIFF") || at(8, b"AIFC")) {
        Some("aiff")
    } else if at(offset, b"MAC ") {
        Some("ape")
    } else if bytes.len() > offset + 3 && byte(offset) == 255 {
        if byte(offset + 1) & 246 == 240 {
            Some("aac")
        } else if byte(offset + 1) & 224 == 224
            && byte(offset + 1) & 6 != 0
            && byte(offset + 1) & 24 != 8
            && ![0, 240].contains(&(byte(offset + 2) & 240))
            && byte(offset + 2) & 12 != 12
        {
            Some("mp3")
        } else {
            None
        }
    } else {
        None
    }
}
pub fn download_name(s: &Song) -> String {
    let artist = if s.artist.trim().is_empty() {
        "未知歌手"
    } else {
        s.artist.trim()
    };
    let name = format!("{} - {artist}", s.title.trim())
        .chars()
        .map(|c| {
            if c.is_control() || "\\/:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(150)
        .collect::<String>()
        .trim()
        .trim_end_matches('.')
        .to_owned();
    // Windows reserved device names remain invalid even with an extension.
    if ["CON", "PRN", "AUX", "NUL"].contains(&name.to_ascii_uppercase().as_str()) {
        format!("_{name}")
    } else {
        name
    }
}
pub fn download(
    r: &Resolved,
    p: &Provider,
    s: &Song,
    directory: &Path,
    cancel: &Cancel,
    progress: impl Fn(u64, Option<u64>),
) -> Result<PathBuf> {
    use std::io::{Read, Write};
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let partial = directory.join(format!(
        ".yyplayer-{}-{}.partial",
        std::process::id(),
        ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut owns_partial = false;
    let result = (|| {
        let mut resp = http::connect(&r.url, "GET", &r.headers, &[], p.timeout_ms, None, cancel)?;
        let length = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        ensure(
            length.is_none_or(|n| n > 0 && n <= MAX_DOWNLOAD),
            "歌曲超过512MB或为空",
        )?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|e| e.to_string())?;
        owns_partial = true;
        let mut reader = resp.body_mut().as_reader();
        let mut total = 0;
        let mut prefix = vec![];
        let mut buffer = [0; 65536];
        let mut last = Instant::now();
        loop {
            cancel.check()?;
            let n = reader.read(&mut buffer).map_err(|_| "下载读取失败")?;
            if n == 0 {
                break;
            }
            total += n as u64;
            ensure(total <= MAX_DOWNLOAD, "歌曲超过512MB")?;
            if prefix.len() < 256 * 1024 {
                prefix.extend_from_slice(&buffer[..n.min(256 * 1024 - prefix.len())]);
            }
            file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            if last.elapsed() > Duration::from_millis(200) {
                progress(total, length);
                last = Instant::now();
            }
        }
        ensure(total > 0 && length.is_none_or(|n| n == total), "下载不完整")?;
        let ext = audio_extension(&prefix).ok_or("无法识别音频文件头，下载未保存")?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        cancel.check()?;
        for i in 1..10000 {
            let name = if i == 1 {
                format!("{}.{ext}", download_name(s))
            } else {
                format!("{} ({i}).{ext}", download_name(s))
            };
            let dest = directory.join(name);
            // Hard link atomically reserves a complete file without overwriting a user's existing song.
            match publish(&partial, &dest) {
                Ok(()) => return Ok(dest),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(format!("下载目录需支持原子文件发布：{e}")),
            }
        }
        Err("重名文件过多".into())
    })();
    if owns_partial {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

fn publish(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<_> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<_> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: both buffers are NUL-terminated and live through this same-directory rename.
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::fs::hard_link(from, to)
    }
}

pub fn read_prefix(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    ensure(
        f.metadata().map_err(|e| e.to_string())?.len() <= MAX_DOWNLOAD,
        "缓存过大",
    )?;
    let mut bytes = vec![];
    (&mut f)
        .take(256 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

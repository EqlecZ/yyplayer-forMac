mod cache;
pub mod config;
mod http;
pub mod proxy;
pub mod repository;
#[cfg(test)]
mod tests;
use config::{Config, Provider, Result};
use repository::{Repository, Song};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
#[derive(Clone)]
pub struct Directories {
    pub api: PathBuf,
    pub download: PathBuf,
    pub lyric: PathBuf,
    pub artwork: PathBuf,
    pub stream: PathBuf,
}
impl Directories {
    pub fn from_settings(s: &player_core::online::OnlineSettings) -> Self {
        let app = crate::services::settings_path()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_owned();
        let exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_owned()))
            .unwrap_or_else(|| PathBuf::from("."));
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(PathBuf::from)
            .unwrap_or_else(|| app.clone());
        let choose = |path: &PathBuf, default: PathBuf| {
            if path.as_os_str().is_empty() {
                default
            } else {
                path.clone()
            }
        };
        Self {
            api: choose(&s.api_directory, exe.join("config/cloud_api")),
            download: choose(&s.download_directory, home.join("Music/YYPlayer")),
            lyric: choose(&s.lyric_directory, app.join("cache/online-lyrics"))
                .join("yyplayer-lyrics"),
            artwork: choose(&s.artwork_directory, app.join("cache/online-artwork"))
                .join("yyplayer-artwork"),
            stream: choose(&s.stream_directory, app.join("cache/online-stream")),
        }
    }
}
pub enum Job {
    Reload(PathBuf),
    Search(Provider, String, String, usize, bool),
    Qualities(Provider, Song, bool),
    Play(Provider, Song, usize, Directories, bool, bool), // quality, directories, download, lyrics allowed
    Lyrics(Provider, Song, PathBuf, Option<String>),
    Artwork(Provider, Song, PathBuf),
    Directory(usize),
    OpenDirectory(PathBuf),
}
pub enum Event {
    Config(Result<Config>),
    Songs(Result<Vec<Song>>),
    Qualities(Vec<(bool, String)>),
    Playing(
        Song,
        Result<(player_core::MediaSource, Option<proxy::Proxy>, String)>,
    ),
    Downloaded(Song, Result<PathBuf>),
    DownloadComplete(PathBuf),
    Lyrics(Song, Result<String>),
    Artwork(Song, Result<Option<(u32, u32, Vec<u8>)>>, Option<u32>),
    Progress(u64, Option<u64>),
    Directory(usize, PathBuf),
    DirectoryOpened(Result<()>),
}
pub struct Reply {
    pub lane: usize,
    pub id: u64,
    pub event: Event,
}
fn artwork_event(song: Song, art: Result<Option<(u32, u32, Vec<u8>)>>) -> Event {
    let color = art
        .as_ref()
        .ok()
        .and_then(|a| a.as_ref())
        .and_then(|(_, _, pixels)| super::music_assets::cover_color(pixels));
    Event::Artwork(song, art, color)
}
struct Work {
    lane: usize,
    cancel: http::Cancel,
    job: Job,
}
pub struct Service {
    commands: crossbeam_channel::Sender<Work>,
    pub replies: crossbeam_channel::Receiver<Reply>,
    generations: [Arc<AtomicU64>; 4],
    workers: Vec<std::thread::JoinHandle<()>>,
    music_allowed: Arc<AtomicBool>,
    lyrics_allowed: Arc<AtomicBool>,
}
impl Service {
    pub fn start() -> Self {
        let (commands, rx) = crossbeam_channel::bounded::<Work>(8);
        let (tx, replies) = crossbeam_channel::bounded(16);
        let generations = std::array::from_fn(|_| Arc::new(AtomicU64::new(0)));
        let repo = Repository::default();
        let music_allowed = Arc::new(AtomicBool::new(false));
        let lyrics_allowed = Arc::new(AtomicBool::new(false));
        let mut workers = vec![];
        for _ in 0..2 {
            let rx = rx.clone();
            let tx = tx.clone();
            let repo = repo.clone();
            let music = music_allowed.clone();
            let lyrics_permitted = lyrics_allowed.clone();
            workers.push(std::thread::spawn(move || {
                while let Ok(mut w) = rx.recv() {
                    w.cancel.permission = match &w.job {
                        Job::Search(_, _, _, _, true) | Job::Lyrics(..) => {
                            Some(lyrics_permitted.clone())
                        }
                        Job::Play(..)
                        | Job::Qualities(..)
                        | Job::Artwork(..)
                        | Job::Search(_, _, _, _, false) => Some(music.clone()),
                        _ => None,
                    };
                    if w.cancel.check().is_err() {
                        continue;
                    }
                    let send = |event| {
                        if w.cancel.check().is_ok() {
                            let _ = tx.send(Reply {
                                lane: w.lane,
                                id: w.cancel.id,
                                event,
                            });
                        }
                    };
                    match w.job {
                        Job::Reload(dir) => {
                            let result = load_configs(&dir);
                            if result.is_ok() {
                                repo.clear_resolved();
                            }
                            send(Event::Config(result));
                        }
                        Job::Search(p, source, q, page, lyrics) => send(Event::Songs(
                            repo.search(&p, &source, &q, page, lyrics, &w.cancel),
                        )),
                        Job::Qualities(p, s, download) => {
                            let options = p
                                .qualities
                                .iter()
                                .map(|q| match repo.resolve(&p, &s, q, download, &w.cancel) {
                                    Ok(r) => (
                                        true,
                                        if r.report == q.id || r.report == q.value {
                                            format!("服务返回此档位；格式以文件为准{}", r.details())
                                        } else {
                                            format!("地址可用；服务未确认此档位{}", r.details())
                                        },
                                    ),
                                    Err(e) => (false, e),
                                })
                                .collect();
                            send(Event::Qualities(options));
                        }
                        Job::Play(p, s, quality, dirs, download, lyrics) => {
                            let resolved = p
                                .qualities
                                .get(quality)
                                .ok_or("音质不存在".into())
                                .and_then(|q| repo.resolve(&p, &s, q, download, &w.cancel));
                            let mut local = None;
                            if download {
                                let result = resolved.and_then(|r| {
                                    repository::download(
                                        &r,
                                        &p,
                                        &s,
                                        &dirs.download,
                                        &w.cancel,
                                        |n, total| send(Event::Progress(n, total)),
                                    )
                                });
                                if let Ok(path) = &result {
                                    local = Some(path.clone());
                                }
                                send(Event::Downloaded(s.clone(), result));
                            } else {
                                let result = resolved.and_then(|r| {
                                    let report = if r.report.is_empty() {
                                        "服务未报告音质".into()
                                    } else {
                                        format!("服务报告：{}{}", r.report, r.details())
                                    };
                                    let key = repository::media_key(&p, &s, &p.qualities[quality])?;
                                    let cached = dirs
                                        .stream
                                        .join("yyplayer-stream")
                                        .join(format!("{key}.audio"));
                                    if let Ok(prefix) = repository::read_prefix(&cached)
                                        && repository::audio_extension(&prefix).is_some()
                                    {
                                        return Ok((
                                            player_core::MediaSource::Local(cached),
                                            None,
                                            format!("{report} · 已缓存"),
                                        ));
                                    }
                                    let proxy = proxy::Proxy::start(
                                        r,
                                        p.timeout_ms,
                                        dirs.stream.clone(),
                                        key,
                                    )?;
                                    Ok((
                                        player_core::MediaSource::Url(proxy.url.clone()),
                                        Some(proxy),
                                        report,
                                    ))
                                });
                                let success = result.is_ok();
                                send(Event::Playing(s.clone(), result));
                                if !success {
                                    continue;
                                }
                            }
                            if w.cancel.check().is_err() {
                                continue;
                            }
                            if lyrics {
                                let mut lyric_cancel = w.cancel.clone();
                                lyric_cancel.permission = Some(lyrics_permitted.clone());
                                let lyric = repo.lyric(&p, &s, &dirs.lyric, &lyric_cancel);
                                if let (Some(path), Ok(text)) = (&local, &lyric) {
                                    let _ = sibling(path, "lrc", text.as_bytes());
                                }
                                send(Event::Lyrics(s.clone(), lyric));
                            }
                            let art = repo.artwork(&p, &s, &dirs.artwork, &w.cancel);
                            if let (Some(path), Ok(Some((width, height, bytes)))) = (&local, &art)
                                && let Some(image) =
                                    image::RgbaImage::from_raw(*width, *height, bytes.clone())
                            {
                                let mut jpg = std::io::Cursor::new(vec![]);
                                if image::DynamicImage::ImageRgba8(image)
                                    .write_to(&mut jpg, image::ImageFormat::Jpeg)
                                    .is_ok()
                                {
                                    let _ = sibling(path, "jpg", &jpg.into_inner());
                                }
                            }
                            send(artwork_event(s, art));
                            if let Some(path) = local {
                                send(Event::DownloadComplete(path));
                            }
                        }
                        Job::Lyrics(p, s, dir, current_key) => {
                            let result = repo.lyric(&p, &s, &dir, &w.cancel).and_then(|text| {
                                w.cancel.check()?;
                                let selected = dir.join("selected");
                                crate::services::atomic_bytes(
                                    &selected.join(format!("{}.lrc", repository::hash(&s.key()))),
                                    text.as_bytes(),
                                )?;
                                if let Some(key) = current_key {
                                    crate::services::atomic_bytes(
                                        &selected.join(format!("{}.lrc", repository::hash(&key))),
                                        text.as_bytes(),
                                    )?;
                                }
                                Ok(text)
                            });
                            send(Event::Lyrics(s, result));
                        }
                        Job::Artwork(p, s, dir) => send(artwork_event(
                            s.clone(),
                            repo.artwork(&p, &s, &dir, &w.cancel).map(|pixels| {
                                pixels.and_then(|(w, h, bytes)| {
                                    image::RgbaImage::from_raw(w, h, bytes).map(|image| {
                                        let image = image::DynamicImage::ImageRgba8(image);
                                        let image = image
                                            .thumbnail(
                                                image.width().min(128),
                                                image.height().min(128),
                                            )
                                            .into_rgba8();
                                        (image.width(), image.height(), image.into_raw())
                                    })
                                })
                            }),
                        )),
                        Job::Directory(kind) => {
                            if let Some(p) = rfd::FileDialog::new()
                                .set_title("选择联网服务目录")
                                .pick_folder()
                            {
                                send(Event::Directory(kind, p));
                            }
                        }
                        Job::OpenDirectory(path) => {
                            send(Event::DirectoryOpened(
                                std::fs::create_dir_all(&path)
                                    .and_then(|()| player_platform::directory::open(&path))
                                    .map_err(|e| format!("无法打开 API 配置文件夹：{e}")),
                            ));
                        }
                    }
                }
            }));
        }
        Self {
            commands,
            replies,
            generations,
            workers,
            music_allowed,
            lyrics_allowed,
        }
    }
    pub fn set_permissions(&self, music: bool, lyrics: bool) {
        self.music_allowed.store(music, Ordering::Release);
        self.lyrics_allowed.store(lyrics, Ordering::Release);
    }
    pub fn cancel(&self, lane: usize) -> u64 {
        self.generations[lane].fetch_add(1, Ordering::AcqRel) + 1
    }
    pub fn submit(&self, lane: usize, id: u64, job: Job) -> Result<()> {
        self.commands
            .try_send(Work {
                lane,
                cancel: http::Cancel {
                    permission: None,
                    generation: self.generations[lane].clone(),
                    id,
                },
                job,
            })
            .map_err(|_| "联网任务繁忙，请稍后再试".into())
    }
    pub fn finish(self) {
        for lane in 0..4 {
            self.cancel(lane);
        }
        drop(self.commands);
        drop(self.replies);
        for w in self.workers {
            let _ = w.join();
        }
    }
}
fn sibling(path: &std::path::Path, extension: &str, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path.with_extension(extension))
        .map_err(|e| e.to_string())?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())
}
pub fn load_configs(directory: &std::path::Path) -> Result<Config> {
    if !directory.exists() {
        let _ = std::fs::create_dir_all(directory);
    }
    let mut files = if directory.is_dir() {
        std::fs::read_dir(directory)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
            })
            .take(18)
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    files.sort();
    if files.len() > 1 {
        files.retain(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_none_or(|s| !s.eq_ignore_ascii_case("example-gd.json"))
        });
    }
    if files.is_empty() {
        return Config::parse(include_bytes!(
            "../../../../config/cloud_api/example-gd.json"
        ));
    }
    config::ensure(files.len() <= 16, "配置文件超过16份")?;
    let mut providers = vec![];
    let mut ids = std::collections::BTreeSet::new();
    for file in files {
        let c = Config::parse(&repository::read_file(&file, 256 * 1024)?)?;
        for p in c.providers {
            config::ensure(ids.insert(p.id.clone()), "多个文件的API id重复")?;
            providers.push(p);
        }
    }
    config::ensure(providers.len() <= 16, "API总数超过16个")?;
    Ok(Config {
        version: 1,
        providers,
    })
}

//! Bounded disk/tag/dialog worker. Replies carry identity so old lyrics never replace a new song.
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::Accessor,
};
use player_core::{audio::EqPreset, lyrics::Lyrics};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, SyncSender, sync_channel},
};

#[derive(Default)]
pub struct Assets {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub info: String,
    pub file_type: String,
    pub bitrate_kbps: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub cover: Option<(u32, u32, Vec<u8>)>,
    pub cover_color: Option<u32>,
    pub lyrics: Lyrics,
    pub warning: String,
}
pub enum Request {
    Read(u64, PathBuf, Option<PathBuf>),
    Lyrics(u64, PathBuf),
    ImportEq,
    ExportEq(EqPreset),
}
pub enum Reply {
    Assets(u64, Box<Assets>),
    Lyrics(u64, PathBuf, PathBuf, Lyrics),
    Eq(EqPreset),
    Message(String),
}
pub struct Service {
    pub commands: SyncSender<Request>,
    pub replies: Receiver<Reply>,
}
impl Service {
    pub fn start() -> Self {
        let (tx, rx) = sync_channel(4);
        let (done, replies) = sync_channel(4);
        std::thread::spawn(move || {
            lofty::config::apply_global_options(
                lofty::config::GlobalOptions::new().allocation_limit(16 * 1024 * 1024),
            );
            while let Ok(request) = rx.recv() {
                let reply = match request {
                    Request::Read(id, path, lyrics) => Some(Reply::Assets(
                        id,
                        Box::new(read_assets(&path, lyrics.as_deref())),
                    )),
                    Request::Lyrics(id, media) => rfd::FileDialog::new()
                        .set_title("导入当前曲目的本地歌词")
                        .add_filter("歌词", &["lrc", "txt"])
                        .pick_file()
                        .map(|path| match read_lyrics(&path) {
                            Ok(lyrics) => Reply::Lyrics(id, media, path, lyrics),
                            Err(e) => Reply::Message(e),
                        }),
                    Request::ImportEq => rfd::FileDialog::new()
                        .set_title("导入 EQ（JSON / Equalizer APO 参数滤波器）")
                        .add_filter("EQ 预设", &["json", "txt"])
                        .pick_file()
                        .map(|path| {
                            match read_text(&path, 256_000).and_then(|text| {
                                EqPreset::import(
                                    &text,
                                    &path.file_stem().unwrap_or_default().to_string_lossy(),
                                )
                            }) {
                                Ok(eq) => Reply::Eq(eq),
                                Err(e) => Reply::Message(e),
                            }
                        }),
                    Request::ExportEq(eq) => rfd::FileDialog::new()
                        .set_title("导出 EQ 预设")
                        .set_file_name("YYPlayer-EQ.json")
                        .add_filter("JSON", &["json"])
                        .save_file()
                        .map(|path| {
                            let result = eq
                                .validate()
                                .and_then(|_| {
                                    serde_json::to_vec_pretty(&eq).map_err(|e| e.to_string())
                                })
                                .and_then(|data| crate::services::atomic_bytes(&path, &data));
                            Reply::Message(match result {
                                Ok(()) => "EQ 预设已导出".into(),
                                Err(e) => format!("导出失败：{e}"),
                            })
                        }),
                };
                if let Some(reply) = reply
                    && done.send(reply).is_err()
                {
                    break;
                }
            }
        });
        Self {
            commands: tx,
            replies,
        }
    }
}
fn read_text(path: &Path, limit: usize) -> Result<String, String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("无法读取 {}：{e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("文件超出大小限制".into());
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        return Ok(encoding_rs::UTF_16LE.decode(&bytes[2..]).0.into_owned());
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        return Ok(encoding_rs::UTF_16BE.decode(&bytes[2..]).0.into_owned());
    }
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s.trim_start_matches('\u{feff}').into()),
        Err(e) => Ok(encoding_rs::GB18030.decode(e.as_bytes()).0.into_owned()),
    }
}
fn read_lyrics(path: &Path) -> Result<Lyrics, String> {
    Lyrics::parse(&read_text(path, 1_000_000)?)
}
fn read_assets(path: &Path, override_lyrics: Option<&Path>) -> Assets {
    let mut result = Assets::default();
    if let Ok(file) = lofty::probe::read_from_path(path) {
        let properties = file.properties();
        result.file_type = format!("{:?}", file.file_type()).to_uppercase();
        result.bitrate_kbps = properties.audio_bitrate();
        result.sample_rate = properties.sample_rate();
        result.bit_depth = properties.bit_depth();
        result.info = format!(
            "文件标签：{} Hz · {} bit · {} 声道 · {} kbps",
            properties
                .sample_rate()
                .map_or("未知".into(), |v| v.to_string()),
            properties
                .bit_depth()
                .map_or("未知".into(), |v| v.to_string()),
            properties
                .channels()
                .map_or("未知".into(), |v| v.to_string()),
            properties
                .audio_bitrate()
                .map_or("未知".into(), |v| v.to_string())
        );
        if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
            result.title = tag.title().map_or(String::new(), |v| v.into_owned());
            result.artist = tag.artist().map_or(String::new(), |v| v.into_owned());
            result.album = tag.album().map_or(String::new(), |v| v.into_owned());
            if let Some(lyrics) = tag.get_string(lofty::tag::ItemKey::Lyrics)
                && let Ok(lyrics) = Lyrics::parse(lyrics)
            {
                result.lyrics = lyrics;
            }
            if let Some(picture) = tag
                .pictures()
                .iter()
                .find(|p| p.pic_type() == lofty::picture::PictureType::CoverFront)
                .or_else(|| tag.pictures().first())
            {
                result.cover = decode_picture(picture.data());
            }
        }
    }
    if result.cover.is_none() {
        let candidates = [
            path.with_extension("jpg"),
            path.with_extension("png"),
            path.with_file_name("cover.jpg"),
            path.with_file_name("cover.png"),
            path.with_file_name("folder.jpg"),
        ];
        for candidate in candidates {
            if let Ok(file) = std::fs::File::open(candidate) {
                let mut bytes = Vec::new();
                if file
                    .take(16 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .is_ok()
                    && let Some(cover) = decode_picture(&bytes)
                {
                    result.cover = Some(cover);
                    break;
                }
            }
        }
    }
    let automatic = path.with_extension("lrc");
    let lyric = override_lyrics.unwrap_or(&automatic);
    match read_lyrics(lyric) {
        Ok(lyrics) => result.lyrics = lyrics,
        Err(e) if lyric.exists() || override_lyrics.is_some() => result.warning = e,
        _ => {}
    }
    result.cover_color = result
        .cover
        .as_ref()
        .and_then(|(_, _, pixels)| cover_color(pixels));
    result
}
/// Bounded, alpha-aware RGB clustering on the asset worker, once per cover.
/// Prefer a substantial chromatic cluster over black/white borders; neutral
/// artwork keeps its actual neutral color. No UI-thread image analysis.
pub(super) fn cover_color(pixels: &[u8]) -> Option<u32> {
    #[derive(Clone, Copy, Default)]
    struct Bin {
        weight: u64,
        rgb: [u64; 3],
        chromatic: bool,
    }
    let mut bins = [Bin::default(); 512];
    let mut neutral = Bin::default();
    let stride = (pixels.len() / 4).div_ceil(16_384).max(1);
    for p in pixels.as_chunks::<4>().0.iter().step_by(stride) {
        if p[3] < 64 {
            continue;
        }
        let high = *p[..3].iter().max().unwrap() as u64;
        let low = *p[..3].iter().min().unwrap() as u64;
        let chroma = high - low;
        let weight = p[3] as u64;
        neutral.weight += weight;
        for (sum, &c) in neutral.rgb.iter_mut().zip(p) {
            *sum += c as u64 * weight;
        }
        if high < 24 || low > 235 {
            continue;
        }
        let key = ((p[0] as usize >> 5) << 6) | ((p[1] as usize >> 5) << 3) | (p[2] as usize >> 5);
        let bin = &mut bins[key];
        let weight = weight * (32 + chroma);
        bin.weight += weight;
        bin.chromatic |= chroma >= 24;
        for (sum, &c) in bin.rgb.iter_mut().zip(p) {
            *sum += c as u64 * weight;
        }
    }
    let bin = bins
        .iter()
        .filter(|b| b.chromatic)
        .max_by_key(|b| b.weight)
        .unwrap_or(&neutral);
    (bin.weight > 0).then(|| {
        let c = bin.rgb.map(|c| (c / bin.weight) as u32);
        (c[0] << 16) | (c[1] << 8) | c[2]
    })
}
pub(super) fn read_thumbnail(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    if let Ok(file) = lofty::probe::read_from_path(path)
        && let Some(tag) = file.primary_tag().or_else(|| file.first_tag())
        && let Some(p) = tag
            .pictures()
            .iter()
            .find(|p| p.pic_type() == lofty::picture::PictureType::CoverFront)
            .or_else(|| tag.pictures().first())
        && let Some(image) = decode_picture_at(p.data(), 64)
    {
        return Some(image);
    }
    for candidate in [
        path.with_extension("jpg"),
        path.with_extension("png"),
        path.with_file_name("cover.jpg"),
        path.with_file_name("cover.png"),
        path.with_file_name("folder.jpg"),
    ] {
        if let Ok(file) = std::fs::File::open(candidate) {
            let mut bytes = Vec::new();
            if file
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .is_ok()
                && let Some(image) = decode_picture_at(&bytes, 64)
            {
                return Some(image);
            }
        }
    }
    None
}
fn decode_picture(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    decode_picture_at(bytes, 1024)
}
fn decode_picture_at(bytes: &[u8], size: u32) -> Option<(u32, u32, Vec<u8>)> {
    if bytes.len() > 16 * 1024 * 1024 {
        return None;
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().ok()?;
    let decoded = if decoded.width() > size || decoded.height() > size {
        decoded.resize(size, size, image::imageops::FilterType::Triangle)
    } else {
        decoded
    }
    .to_rgba8();
    Some((decoded.width(), decoded.height(), decoded.into_raw()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cover_palette_ignores_borders_and_transparency_and_handles_neutrals() {
        let mut pixels = [255, 255, 255, 255].repeat(200);
        pixels.extend([212, 68, 84, 255].repeat(100));
        pixels.extend([0, 0, 0, 255].repeat(200));
        pixels.extend([0, 255, 0, 0].repeat(1000));
        assert_eq!(cover_color(&pixels), Some(0xd44454));
        assert_eq!(cover_color(&[90, 90, 90, 255].repeat(10)), Some(0x5a5a5a));
        assert_eq!(cover_color(&[255, 255, 255, 255]), Some(0xffffff));
        assert_eq!(cover_color(&[0, 0, 0, 255]), Some(0));
        assert_eq!(cover_color(&[255, 0, 0, 0]), None);
        assert_eq!(cover_color(&[]), None);
        assert_eq!(cover_color(&[12, 20]), None);
        let large = [40, 100, 210, 255].repeat(1_048_576);
        assert_eq!(cover_color(&large), Some(0x2864d2));
    }
    #[test]
    fn thumbnails_preserve_aspect_and_reject_invalid_images() {
        let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            320,
            160,
            image::Rgba([36, 96, 180, 255]),
        ));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let (w, h, pixels) = decode_picture_at(bytes.get_ref(), 64).unwrap();
        assert_eq!((w, h, pixels.len()), (64, 32, 64 * 32 * 4));
        assert!(decode_picture_at(b"broken picture", 64).is_none());
        assert!(decode_picture_at(&vec![0; 16 * 1024 * 1024 + 1], 64).is_none());
        let path =
            std::env::temp_dir().join(format!("yy-thumbnail-{}.missing", std::process::id()));
        let sidecar = path.with_extension("png");
        std::fs::write(&sidecar, bytes.get_ref()).unwrap();
        assert_eq!(read_thumbnail(&path).unwrap().0, 64);
        std::fs::remove_file(sidecar).unwrap();
    }
    #[test]
    fn lyrics_encodings_and_size_limit() {
        let path = std::env::temp_dir().join(format!("yy-lrc-{}.txt", std::process::id()));
        std::fs::write(
            &path,
            [
                0xff, 0xfe, b'[', 0, b'0', 0, b'0', 0, b':', 0, b'0', 0, b'1', 0, b']', 0, b'A', 0,
            ],
        )
        .unwrap();
        assert_eq!(read_lyrics(&path).unwrap().lines[0].text, "A");
        assert!(read_text(&path, 2).is_err());
        std::fs::remove_file(path).unwrap();
    }
}

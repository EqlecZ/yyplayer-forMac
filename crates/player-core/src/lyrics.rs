//! Bounded LRC parsing, millisecond timestamps and stable duplicate timestamps.
use serde::{Deserialize, Serialize};

/// Display only: never changes timestamps, playback or the saved lyric source.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LyricStyle {
    pub album_color: bool,
    pub bilingual: bool,
    pub line_gap: u16,
    pub inactive_size: u16,
    pub inactive_opacity: u8,
    pub inactive_weight: u16,
    pub active_size: u16,
    pub active_weight: u16,
    pub secondary_weight: u16,
    pub bilingual_gap: u16,
}
impl Default for LyricStyle {
    fn default() -> Self {
        Self {
            album_color: true,
            bilingual: true,
            line_gap: 24,
            inactive_size: 20,
            inactive_opacity: 55,
            inactive_weight: 400,
            active_size: 28,
            active_weight: 700,
            secondary_weight: 400,
            bilingual_gap: 8,
        }
    }
}
impl LyricStyle {
    pub fn validate(&self) -> Result<(), String> {
        if self.line_gap > 80 || self.bilingual_gap > 40 {
            return Err("歌词行距应为 0–80 px，双语间距应为 0–40 px".into());
        }
        if !(12..=48).contains(&self.inactive_size) || !(12..=64).contains(&self.active_size) {
            return Err("普通行字号应为 12–48 px，当前行字号应为 12–64 px".into());
        }
        if self.inactive_opacity > 100 {
            return Err("歌词不透明度应为 0–100%".into());
        }
        if [
            self.inactive_weight,
            self.active_weight,
            self.secondary_weight,
        ]
        .into_iter()
        .any(|w| !(100..=900).contains(&w) || w % 100 != 0)
        {
            return Err("歌词字重应为 100–900，步长 100".into());
        }
        Ok(())
    }
}

/// A spaced slash is a lyric language separator. Bare slashes in URLs, names
/// and dates retain their original meaning. Empty sides are not translations.
pub fn bilingual_parts(text: &str) -> (&str, &str) {
    if let Some((primary, secondary)) = text.split_once(" / ") {
        let (primary, secondary) = (primary.trim(), secondary.trim());
        if !primary.is_empty() && !secondary.is_empty() {
            return (primary, secondary);
        }
    }
    (text, "")
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LyricLine {
    pub milliseconds: i64,
    pub text: String,
}
#[derive(Clone, Debug, Default)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    pub plain: String,
}
impl Lyrics {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 1_000_000 {
            return Err("歌词文件超过 1MB".into());
        }
        let mut result = Self::default();
        let mut offset = 0i64;
        for raw in text.trim_start_matches('\u{feff}').lines() {
            let mut line = raw.trim();
            let mut times = Vec::new();
            while let Some(rest) = line.strip_prefix('[') {
                let Some((tag, tail)) = rest.split_once(']') else {
                    break;
                };
                line = tail;
                if let Some(value) = tag.strip_prefix("offset:") {
                    offset = value.trim().parse().map_err(|_| "非法 LRC offset")?;
                    if !(-600_000..=600_000).contains(&offset) {
                        return Err("LRC offset 超出范围".into());
                    }
                } else if let Some((minutes, seconds)) = tag.split_once(':')
                    && let (Ok(m), Ok(s)) = (minutes.parse::<i64>(), seconds.parse::<f64>())
                {
                    if !(0..=10000).contains(&m) || !s.is_finite() || !(0.0..60.0).contains(&s) {
                        return Err("非法 LRC 时间戳".into());
                    }
                    times.push(m * 60_000 + (s * 1000.0).round() as i64);
                }
            }
            if !times.is_empty() {
                for milliseconds in times {
                    result.lines.push(LyricLine {
                        milliseconds,
                        text: line.trim().into(),
                    });
                }
            } else if !line.trim().is_empty() {
                result.plain.push_str(line);
                result.plain.push('\n');
            }
            if result.lines.len() > 10000 {
                return Err("歌词行数超过 10000".into());
            }
        }
        for line in &mut result.lines {
            line.milliseconds = (line.milliseconds + offset).max(0);
        }
        result.lines.sort_by_key(|line| line.milliseconds);
        Ok(result)
    }
    pub fn active(&self, position_ms: i64, user_offset_ms: i64) -> Option<usize> {
        self.lines
            .partition_point(|line| line.milliseconds <= position_ms.saturating_add(user_offset_ms))
            .checked_sub(1)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_separator_preserves_source_and_timing() {
        assert_eq!(
            bilingual_parts("Stay with me / 请留在我身边"),
            ("Stay with me", "请留在我身边")
        );
        assert_eq!(bilingual_parts("  夜の風 / 晚风  "), ("夜の風", "晚风"));
        for s in [
            "AC/DC",
            "https://example.com/lyrics",
            "2026/10/11",
            "Hello / ",
            " / 世界",
            "单语歌词",
        ] {
            assert_eq!(bilingual_parts(s), (s, ""));
        }
        let l = Lyrics::parse("[offset:150]\n[00:02]Original / 翻译").unwrap();
        assert_eq!(l.lines[0].text, "Original / 翻译");
        assert_eq!(l.lines[0].milliseconds, 2150);
        assert_eq!(l.active(2150, 0), Some(0));
    }
    #[test]
    fn display_settings_migrate_roundtrip_and_reject_bad_values() {
        let old: crate::settings::Settings = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert_eq!(old.lyrics, LyricStyle::default());
        let mut style: LyricStyle = serde_json::from_str(r#"{"inactive_size":32}"#).unwrap();
        assert_eq!(style.active_size, 28);
        style.line_gap = 80;
        style.active_size = 64;
        style.inactive_opacity = 0;
        style.secondary_weight = 900;
        style.bilingual_gap = 40;
        style.validate().unwrap();
        let restored: LyricStyle =
            serde_json::from_str(&serde_json::to_string(&style).unwrap()).unwrap();
        assert_eq!(style, restored);
        for invalid in [
            r#"{"line_gap":81}"#,
            r#"{"inactive_size":11}"#,
            r#"{"active_size":65}"#,
            r#"{"inactive_opacity":101}"#,
            r#"{"active_weight":550}"#,
            r#"{"secondary_weight":1000}"#,
            r#"{"bilingual_gap":41}"#,
        ] {
            let style: LyricStyle = serde_json::from_str(invalid).unwrap();
            assert!(style.validate().is_err(), "{invalid}");
        }
    }
    #[test]
    fn duplicate_offset_seek_and_bounds() {
        let lyrics =
            Lyrics::parse("[offset:-100]\n[00:02.50][00:01.20]Second\n[00:01.20]Translation")
                .unwrap();
        assert_eq!(lyrics.lines[0].milliseconds, 1100);
        assert_eq!(lyrics.active(1100, 0), Some(1));
        assert_eq!(lyrics.active(100, 0), None);
        assert!(Lyrics::parse("[00:99]bad").is_err());
        assert!(Lyrics::parse("[offset:99999999999]").is_err());
    }
}

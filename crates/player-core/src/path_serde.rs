//! Lossless JSON path identities. Ordinary UTF-8 paths keep the existing schema.
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use std::{collections::BTreeMap, path::PathBuf};

// NUL cannot occur in a real OS path, so this JSON-only marker cannot collide
// with any literal filename from an existing version-1 UTF-8 configuration.
const PREFIX: &str = "\0yyplayer-path-v1~";
fn encode(path: &std::path::Path) -> Result<String, String> {
    if let Some(text) = path.to_str() {
        if text.contains('\0') {
            return Err("路径不能包含 NUL".into());
        }
        return Ok(text.to_owned());
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = path.as_os_str().as_bytes();
        if bytes.contains(&0) {
            return Err("路径不能包含 NUL".into());
        }
        let mut value = format!("{PREFIX}unix:");
        for byte in bytes {
            use std::fmt::Write;
            write!(value, "{byte:02x}").unwrap();
        }
        Ok(value)
    }
    #[cfg(not(unix))]
    Err("路径不是有效 UTF-8".into())
}
fn decode(value: String) -> Result<PathBuf, String> {
    let Some(encoded) = value.strip_prefix(PREFIX) else {
        if value.contains('\0') {
            return Err("路径不能包含 NUL".into());
        }
        return Ok(value.into());
    };
    #[cfg(not(unix))]
    let _ = encoded;
    #[cfg(unix)]
    if let Some(hex) = encoded.strip_prefix("unix:") {
        use std::os::unix::ffi::OsStringExt;
        if !hex.len().is_multiple_of(2) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("原始路径字节编码损坏".into());
        }
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>();
        if bytes.contains(&0) {
            return Err("路径不能包含 NUL".into());
        }
        return Ok(std::ffi::OsString::from_vec(bytes).into());
    }
    Err("不支持此平台的原始路径编码".into())
}
#[derive(Ord, PartialOrd, Eq, PartialEq)]
struct Identity(PathBuf);
impl Serialize for Identity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        encode(&self.0)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Identity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        decode(String::deserialize(deserializer)?)
            .map(Self)
            .map_err(D::Error::custom)
    }
}
pub fn serialize<S: Serializer>(path: &std::path::Path, serializer: S) -> Result<S::Ok, S::Error> {
    Identity(path.to_owned()).serialize(serializer)
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    Identity::deserialize(deserializer).map(|p| p.0)
}
pub mod vec {
    use super::*;
    pub fn serialize<S: Serializer>(paths: &[PathBuf], serializer: S) -> Result<S::Ok, S::Error> {
        paths
            .iter()
            .cloned()
            .map(Identity)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<PathBuf>, D::Error> {
        Vec::<Identity>::deserialize(deserializer).map(|v| v.into_iter().map(|p| p.0).collect())
    }
}
pub mod option {
    use super::*;
    pub fn serialize<S: Serializer>(
        path: &Option<PathBuf>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        path.clone().map(Identity).serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<PathBuf>, D::Error> {
        Option::<Identity>::deserialize(deserializer).map(|v| v.map(|p| p.0))
    }
}
pub mod map {
    use super::*;
    pub fn serialize<S: Serializer, T: Serialize>(
        paths: &BTreeMap<PathBuf, T>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        paths
            .iter()
            .map(|(p, v)| (Identity(p.clone()), v))
            .collect::<BTreeMap<_, _>>()
            .serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<PathBuf, T>, D::Error> {
        BTreeMap::<Identity, T>::deserialize(deserializer)
            .map(|v| v.into_iter().map(|(p, v)| (p.0, v)).collect())
    }
}
pub mod path_map {
    use super::*;
    pub fn serialize<S: Serializer>(
        paths: &BTreeMap<PathBuf, PathBuf>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        paths
            .iter()
            .map(|(p, v)| (Identity(p.clone()), Identity(v.clone())))
            .collect::<BTreeMap<_, _>>()
            .serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<PathBuf, PathBuf>, D::Error> {
        BTreeMap::<Identity, Identity>::deserialize(deserializer)
            .map(|v| v.into_iter().map(|(p, v)| (p.0, v.0)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_and_reserved_names_roundtrip() {
        for name in [
            "中文/🎵 a%.wav",
            "~yyplayer-path-v1~unix:ff",
            "~yyplayer-path-v1~utf8:foo",
        ] {
            assert_eq!(decode(name.to_owned()).unwrap(), PathBuf::from(name));
            assert_eq!(
                decode(encode(std::path::Path::new(name)).unwrap()).unwrap(),
                PathBuf::from(name)
            );
        }
        assert!(decode(format!("{PREFIX}unix:f")).is_err());
        assert!(decode(format!("{PREFIX}unix:00")).is_err());
        assert!(encode(std::path::Path::new("invalid\0path")).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn settings_and_index_keep_non_utf8_identity() {
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_vec(b"/music/\xff %.wav".to_vec()));
        let mut settings = crate::settings::Settings::default();
        settings.recent.push(path.clone());
        settings.library.files.push(path.clone());
        settings.files.insert(path.clone(), Default::default());
        settings
            .audio
            .lyric_files
            .insert(path.clone(), path.clone());
        settings.appearance.library_background.file = Some(path.clone());
        let json = serde_json::to_vec(&settings).unwrap();
        let restored: crate::settings::Settings = serde_json::from_slice(&json).unwrap();
        assert_eq!(restored.recent[0], path);
        assert_eq!(restored.library.files[0], path);
        assert!(restored.files.contains_key(&path));
        assert_eq!(restored.audio.lyric_files[&path], path);
        assert_eq!(
            restored.appearance.library_background.file,
            Some(path.clone())
        );
        let song = crate::library::Song {
            path: path.clone(),
            title: "test".into(),
            artist: "".into(),
            album: "".into(),
            seconds: 0,
        };
        let restored: crate::library::Song =
            serde_json::from_slice(&serde_json::to_vec(&song).unwrap()).unwrap();
        assert_eq!(restored.path, path);
    }
}

use super::{
    config::*,
    http::{self, Cancel},
    repository::*,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
fn config() -> Config {
    Config::parse(include_bytes!(
        "../../../../config/cloud_api/example-gd.json"
    ))
    .unwrap()
}
fn cancel() -> Cancel {
    Cancel {
        generation: Arc::new(AtomicU64::new(0)),
        id: 0,
        permission: None,
    }
}
#[test]
fn android_json_golden_and_rejections() {
    let c = config();
    assert_eq!(
        c.providers[0]
            .sources
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["netease", "joox", "bilibili"]
    );
    assert_eq!(
        c.providers[0]
            .qualities
            .iter()
            .map(|q| q.value.as_str())
            .collect::<Vec<_>>(),
        ["128", "192", "320", "740", "999"]
    );
    let original = serde_json::from_slice::<serde_json::Value>(include_bytes!(
        "../../../../config/cloud_api/example-gd.json"
    ))
    .unwrap();
    for key in [
        "version",
        "unsafe-header",
        "http",
        "host",
        "unknown-variable",
        "duplicate-quality",
        "reserved-variable",
        "range",
        "get-body",
        "depth",
    ] {
        let mut v = original.clone();
        match key {
            "version" => v["version"] = 2.into(),
            "unsafe-header" => v["providers"][0]["headers"]["X-Test"] = "unsafe\nvalue".into(),
            "http" => v["providers"][0]["baseUrl"] = "http://example.com".into(),
            "host" => v["providers"][0]["search"]["url"] = "https://other.example.com/api".into(),
            "unknown-variable" => {
                v["providers"][0]["search"]["parameters"]["secret"] = "{unconfigured}".into()
            }
            "duplicate-quality" => v["providers"][0]["qualities"][1]["id"] = "128".into(),
            "reserved-variable" => v["providers"][0]["variables"]["id"] = "evil".into(),
            "range" => v["providers"][0]["headers"]["Range"] = "bytes=0-".into(),
            "get-body" => v["providers"][0]["search"]["body"] = serde_json::json!({"secret":true}),
            "depth" => {
                let mut deep = serde_json::Value::Null;
                for _ in 0..35 {
                    deep = serde_json::json!([deep]);
                }
                v["deep"] = deep;
            }
            _ => unreachable!(),
        }
        assert!(
            Config::parse(&serde_json::to_vec(&v).unwrap()).is_err(),
            "{key}"
        );
    }
}
#[test]
fn templates_nested_json_and_paths() {
    let vars = BTreeMap::from([
        ("query".into(), "中文 & \"quote\"".into()),
        ("quality".into(), "999".into()),
    ]);
    let rendered = body(
        &serde_json::json!({"array":["{query}",1,true,null],"q":{"value":"{quality}"}}),
        &vars,
    )
    .unwrap();
    assert_eq!(rendered["array"][0], vars["query"]);
    assert_eq!(rendered["array"][1], 1);
    assert!(rendered["array"][2].as_bool().unwrap());
    assert_eq!(text(&rendered, "q.value"), "999");
    let url = template("https://example.com/{query}", &vars, true).unwrap();
    assert!(!url.contains('中'));
    assert!(url.contains("%26"));
    assert_eq!(
        text(&serde_json::json!({"a":["one","two"]}), "a"),
        "one / two"
    );
}
#[test]
fn formats_quality_filter_and_translation() {
    assert_eq!(audio_extension(b"RIFFxxxxWAVE"), Some("wav"));
    assert_eq!(audio_extension(b"fLaC"), Some("flac"));
    assert_eq!(audio_extension(b"<html>error</html>"), None);
    assert_eq!(audio_extension(b"OggSxxOpusHead"), Some("opus"));
    assert_eq!(audio_extension(b"ID3\0\0\0\xff\0\0\0"), None);
    let mut song = Song {
        source: "netease".into(),
        qualities: Some(["320".into()].into_iter().collect()),
        ..Default::default()
    };
    let c = config();
    assert!(eligible(&song, &c.providers[0].qualities[2]));
    assert!(!eligible(&song, &c.providers[0].qualities[4]));
    song.title = "A/B:测试".into();
    song.artist = "Singer".into();
    assert_eq!(download_name(&song), "A_B_测试 - Singer");
    let merged =
        merge_lyrics("[00:01.00]One\n[00:02.00]Two", "[00:01.00]一\n[00:03.00]三").unwrap();
    assert!(merged.contains("One / 一"));
    assert!(!merged.contains('三'));
}
#[test]
fn cancelled_generation_rejects_work() {
    let c = cancel();
    assert!(c.check().is_ok());
    c.generation.store(1, Ordering::Release);
    assert!(c.check().is_err());
}
#[test]
fn cache_identity_tracks_configuration_quality_and_variables() {
    let mut p = config().providers.remove(0);
    let mut s = Song {
        id: "song".into(),
        source: "netease".into(),
        ..Default::default()
    };
    let first = media_key(&p, &s, &p.qualities[0]).unwrap();
    assert_ne!(first, media_key(&p, &s, &p.qualities[1]).unwrap());
    p.qualities.swap(0, 1);
    assert_ne!(first, media_key(&p, &s, &p.qualities[0]).unwrap());
    let configured = media_key(&p, &s, &p.qualities[0]).unwrap();
    s.variables.insert("edition".into(), "new".into());
    assert_ne!(configured, media_key(&p, &s, &p.qualities[0]).unwrap());
}
#[test]
#[ignore = "Requires scripts/Serve-OnlineFixtures.py and explicit localhost QA CA"]
fn https_end_to_end() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/online-qa");
    let mut p = super::load_configs(&dir.join("api"))
        .unwrap()
        .providers
        .remove(0);
    let repo = Repository::default();
    let c = cancel();
    let q = "中文 & \"搜索\"";
    let songs = repo.search(&p, "netease", q, 1, false, &c).unwrap();
    assert_eq!(songs.len(), 2);
    assert_eq!(songs[1].title, q);
    let candidates = repo.search(&p, "netease", q, 1, true, &c).unwrap();
    assert_eq!(candidates[0].title, q);
    let mut song = songs[1].clone();
    song.id = "fallback".into();
    assert!(repo.resolve(&p, &song, &p.qualities[4], false, &c).is_err());
    assert_eq!(
        repo.resolve(&p, &song, &p.qualities[2], false, &c)
            .unwrap()
            .report,
        "320"
    );
    song.id = "mislabelled".into();
    song.title = "自有格式验收".into();
    let r = repo.resolve(&p, &song, &p.qualities[4], true, &c).unwrap();
    let output = dir.join("unit-download");
    let a = download(&r, &p, &song, &output, &c, |_, _| {}).unwrap();
    let b = download(&r, &p, &song, &output, &c, |_, _| {}).unwrap();
    assert_ne!(a, b);
    assert_eq!(a.extension().unwrap(), "flac");
    assert_eq!(audio_extension(&std::fs::read(&a).unwrap()), Some("flac"));
    let lyric = repo
        .lyric(&p, &songs[1], &dir.join("unit-lyrics"), &c)
        .unwrap();
    assert!(lyric.contains("原创测试歌词"));
    let artifact = dir.join("unit-artwork").join(format!(
        "{}.jpg",
        hash(&format!("{}:{}", songs[1].key(), songs[1].pic_id))
    ));
    if artifact.is_file() {
        std::fs::remove_file(artifact).unwrap();
    }
    let art = repo
        .artwork(&p, &songs[1], &dir.join("unit-artwork"), &c)
        .unwrap()
        .unwrap();
    assert_eq!((art.0, art.1), (500, 500));
    let headers = Strings::from([("Authorization".into(), "fixture-secret".into())]);
    let bytes = http::read(
        http::connect(
            "https://localhost:18443/redirect",
            "GET",
            &headers,
            &[],
            p.timeout_ms,
            None,
            &c,
        )
        .unwrap(),
        2000,
        &c,
    )
    .unwrap();
    assert!(String::from_utf8(bytes).unwrap().contains("false"));
    let r = repo
        .resolve(&p, &songs[1], &p.qualities[0], false, &c)
        .unwrap();
    let proxy = super::proxy::Proxy::start(
        r,
        p.timeout_ms,
        dir.join("unit-stream"),
        hash("fixture-stream"),
    )
    .unwrap();
    let mut response = ureq::get(&proxy.url)
        .header("Range", "bytes=128-255")
        .call()
        .unwrap();
    assert_eq!(response.status().as_u16(), 206);
    let bytes = response.body_mut().read_to_vec().unwrap();
    assert_eq!(bytes.len(), 128);
    proxy.finish();
    let params = Strings::from([("query".into(), "中文 & \"JSON\"".into())]);
    for encoding in ["form", "json"] {
        let endpoint = Endpoint {
            url: Some("https://localhost:18443/echo".into()),
            method: "POST".into(),
            encoding: encoding.into(),
            parameters: Strings::from([("name".into(), "{query}".into())]),
            headers: Strings::new(),
            body: if encoding == "json" {
                Some(serde_json::json!({"nested":["{query}",12,true,null]}))
            } else {
                None
            },
            response: serde_json::json!({"mode":"json"}),
        };
        let result = repo
            .call(&p, &endpoint, &params, &Strings::new(), &c)
            .unwrap();
        if encoding == "json" {
            assert_eq!(result["received"]["nested"][0], params["query"]);
            assert_eq!(result["received"]["nested"][1], 12);
        } else {
            assert_eq!(result["received"]["name"][0], params["query"]);
        }
    }
    assert!(
        http::connect(
            "https://localhost:18443/redirect",
            "POST",
            &Strings::new(),
            b"{}",
            p.timeout_ms,
            None,
            &c
        )
        .is_err()
    );
    let mut bad = repo
        .resolve(&p, &songs[1], &p.qualities[0], true, &c)
        .unwrap();
    bad.url = "https://localhost:18443/invalid.wav".into();
    assert!(download(&bad, &p, &songs[1], &output, &c, |_, _| {}).is_err());
    bad.url = "https://localhost:18443/slow.wav".into();
    let aborted = cancel();
    let cancel_generation = aborted.generation.clone();
    assert!(
        download(&bad, &p, &songs[1], &output, &aborted, |_, _| {
            cancel_generation.store(1, Ordering::Release);
        })
        .is_err()
    );
    assert!(
        output
            .read_dir()
            .unwrap()
            .flatten()
            .all(|e| e.path().extension().is_none_or(|s| s != "partial"))
    );
    p.requests_per_window = 1;
    p.id = "limit-test".into();
    repo.search(&p, "netease", "limit", 1, false, &c).unwrap();
    assert!(
        repo.search(&p, "netease", "limit", 2, false, &c)
            .unwrap_err()
            .contains("频繁")
    );
    assert!(
        !std::fs::read_dir(output)
            .unwrap()
            .flatten()
            .any(|e| e.path().extension().is_some_and(|s| s == "partial"))
    );
}

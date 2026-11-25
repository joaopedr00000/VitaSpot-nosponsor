//! Independent Free-mode source resolver. Uses Piped's public API, not
//! Spotify's protected audio endpoints. Never forwards Spotify credentials.
use std::{fs::File, io::Write, path::PathBuf, time::Duration};
use reqwest::{Client, Url};
use serde_json::Value;
use crate::{model::Item, state::Backend};

const DEFAULTS: &[&str] = &["https://pipedapi.kavin.rocks", "https://pipedapi.adminforge.de"];
const MAX_JSON: usize = 2 * 1024 * 1024;
const MAX_AUDIO: usize = 32 * 1024 * 1024;
pub type Result<T> = std::result::Result<T, &'static str>;
pub const NETWORK: &str = "Free audio source connection failed.";
pub const NO_MATCH: &str = "No matching public recording found.";
pub const NO_FORMAT: &str = "No compatible AAC stream found.";
pub const FILE_ERROR: &str = "Could not write temporary audio.";
pub const AUDIO_ERROR: &str = "Free audio decoding failed.";
pub const OUTPUT_ERROR: &str = "Free audio output failed.";
pub const LIMIT_ERROR: &str = "Free audio file is too large.";

pub struct TempAudio(pub PathBuf);
impl Drop for TempAudio {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

fn endpoints(backend: &Backend) -> Vec<String> {
    // A user-supplied HTTPS instance replaces defaults, never silently
    // sends searches to an extra provider. No credentials in URLs.
    let path = backend.config.path("free-source.txt");
    if let Ok(text) = std::fs::read_to_string(path) {
        let value = text.trim().trim_end_matches('/');
        if let Ok(url) = Url::parse(value) {
            if url.scheme() == "https" && url.host_str().is_some()
                && url.username().is_empty() && url.password().is_none()
                && url.query().is_none() && url.fragment().is_none() {
                return vec![value.to_owned()];
            }
        }
        log::warn!("Free source configuration invalid");
        return Vec::new();
    }
    DEFAULTS.iter().map(|s| (*s).to_owned()).collect()
}
pub fn cleanup(backend: &Backend) {
    if let Ok(entries) = std::fs::read_dir(backend.config.path("tmp")) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("free-audio-") && name.ends_with(".m4a") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}
fn normalize(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).filter(|c| c.is_alphanumeric()).collect()
}
fn video_id(url: &str) -> Option<&str> {
    let id = url.strip_prefix("/watch?v=")?.split('&').next()?;
    (id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')).then_some(id)
}
fn match_score(track: &Item, candidate: &Value) -> Option<u64> {
    if candidate["type"].as_str().is_some_and(|v| v != "stream")
        || candidate["isShort"].as_bool() == Some(true)
        || candidate["isLive"].as_bool() == Some(true) { return None; }
    let expected = normalize(&track.title);
    let artist = normalize(track.subtitle.split(", ").next().unwrap_or(""));
    let title = normalize(candidate["title"].as_str()?);
    let uploader = normalize(candidate["uploader"].as_str().unwrap_or(""));
    let duration = candidate["duration"].as_u64()?.checked_mul(1000)?;
    if expected.is_empty() || artist.is_empty() || track.duration_ms == 0 { return None; }
    let difference = duration.abs_diff(track.duration_ms as u64);
    if difference > 8000 || !uploader.contains(&artist) { return None; }
    // Reject covers, remixes and live variants unless the Spotify title
    // explicitly identifies that same version. Don't match by duration alone.
    let exact = title == expected;
    let prefixed = title == format!("{artist}{expected}");
    let suffixed = ["officialaudio", "officialmusicvideo", "officialvideo", "lyrics"]
        .iter().any(|suffix| title == format!("{expected}{suffix}") || title == format!("{artist}{expected}{suffix}"));
    if !(exact || prefixed || suffixed) { return None; }
    Some(100_000 - difference - if exact { 0 } else { 10_000 })
}
async fn json(client: &Client, url: String, query: &[(&str, &str)]) -> Result<Value> {
    let mut response = client.get(url).query(query).timeout(Duration::from_secs(15))
        .send().await.map_err(|e| { log::warn!("Free source request failed: timeout={} connect={} request={}", e.is_timeout(), e.is_connect(), e.is_request()); NETWORK })?;
    if !response.status().is_success() {
        log::warn!("Free source API HTTP {}", response.status());
        return Err(NETWORK);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| NETWORK)? {
        if body.len().saturating_add(chunk.len()) > MAX_JSON { return Err(NETWORK); }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|e| { log::warn!("Free source response is not JSON: {e}"); NETWORK })
}
fn select_audio(streams: &Value) -> Option<&str> {
    streams["audioStreams"].as_array()?.iter().filter(|s| {
        s["mimeType"].as_str().is_some_and(|m| m.starts_with("audio/mp4"))
            && s["codec"].as_str().is_some_and(|c| c == "mp4a.40.2")
            && s["url"].as_str().is_some_and(|u| Url::parse(u).is_ok_and(|p| p.scheme() == "https" && p.host_str().is_some() && p.username().is_empty() && p.password().is_none()))
    }).max_by_key(|s| s["bitrate"].as_u64().unwrap_or(0))
        .and_then(|s| s["url"].as_str())
}
async fn download(client: &Client, backend: &Backend, url: &str) -> Result<TempAudio> {
    let mut response = client.get(url).timeout(Duration::from_secs(90)).send().await.map_err(|_| NETWORK)?;
    if response.status() != reqwest::StatusCode::OK {
        log::warn!("Free source media HTTP {}", response.status());
        return Err(NETWORK);
    }
    if response.content_length().is_some_and(|n| n > MAX_AUDIO as u64) { return Err(LIMIT_ERROR); }
    let mime = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
    if !(mime.starts_with("audio/") || mime.starts_with("video/mp4") || mime.starts_with("application/octet-stream")) {
        log::warn!("Free source media content type rejected");
        return Err(NO_FORMAT);
    }
    let temp = TempAudio(PathBuf::from(backend.config.path(&format!("tmp/free-audio-{}-{}.m4a", crate::util::now_ms_inner(), backend.next_request_id()))));
    let mut file = File::create(&temp.0).map_err(|_| FILE_ERROR)?;
    let mut size = 0usize;
    while let Some(chunk) = response.chunk().await.map_err(|_| NETWORK)? {
        size = size.saturating_add(chunk.len());
        if size > MAX_AUDIO { return Err(LIMIT_ERROR); }
        file.write_all(&chunk).map_err(|_| FILE_ERROR)?;
    }
    if size < 64 { return Err(NO_FORMAT); }
    file.flush().map_err(|_| FILE_ERROR)?;
    drop(file);
    log::info!("Free source: received {size} M4A bytes");
    Ok(temp)
}
pub async fn resolve(backend: &Backend, track: &Item) -> Result<TempAudio> {
    let client = Client::builder().https_only(true).redirect(reqwest::redirect::Policy::limited(4))
        .connect_timeout(Duration::from_secs(8)).user_agent("VitaSpot/Free").build().map_err(|_| NETWORK)?;
    let query = format!("{} {}", track.subtitle, track.title);
    let mut last = NETWORK;
    for endpoint in endpoints(backend) {
        let host = Url::parse(&endpoint).ok().and_then(|url| url.host_str().map(str::to_owned)).unwrap_or_default();
        log::info!("Free source: querying {host}");
        let result = async {
            let search = json(&client, format!("{endpoint}/search"), &[("q", &query), ("filter", "music_songs")]).await?;
            let mut candidates: Vec<_> = search["items"].as_array().ok_or(NETWORK)?.iter()
                .filter_map(|v| Some((match_score(track, v)?, video_id(v["url"].as_str()?)?))).collect();
            candidates.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
            log::info!("Free source: search returned {} matching recordings", candidates.len());
            if candidates.is_empty() { return Err(NO_MATCH); }
            let mut error = NO_FORMAT;
            for (_, id) in candidates.into_iter().take(2) {
                let streams = match json(&client, format!("{endpoint}/streams/{id}"), &[]).await {
                    Ok(v) => v, Err(e) => { error = e; continue; }
                };
                if streams["livestream"].as_bool() == Some(true) { continue; }
                // Recheck the returned recording; search results can be stale.
                let Some(_) = match_score(track, &streams) else { error = NO_MATCH; continue; };
                let Some(url) = select_audio(&streams) else { log::warn!("Free source: matched recording has no supported AAC-LC stream"); continue; };
                log::info!("Free source: matched recording {id}; AAC-LC/M4A");
                match download(&client, backend, url).await {
                    Ok(file) => return Ok(file), Err(e) => error = e,
                }
            }
            Err(error)
        }.await;
        match result {
            Ok(audio) => return Ok(audio),
            Err(e) => { log::warn!("Free source {host}: {e}"); last = e; }
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matching_rejects_wrong_recordings() {
        let track = Item { title: "Example".into(), subtitle: "Artist".into(), duration_ms: 180000, ..Default::default() };
        let good = serde_json::json!({"type":"stream","title":"Example","uploader":"Artist - Topic","duration":180,"url":"/watch?v=abcdefghijk"});
        assert!(match_score(&track, &good).is_some());
        let mut cover = good.clone(); cover["title"] = "Example (Cover)".into();
        assert!(match_score(&track, &cover).is_none());
        let mut wrong = good.clone(); wrong["duration"] = 220.into();
        assert!(match_score(&track, &wrong).is_none());
        assert_eq!(video_id("/watch?v=abcdefghijk"), Some("abcdefghijk"));
        assert_eq!(video_id("/watch?v=../../"), None);
    }
    #[test]
    fn select_only_supported_aac() {
        let data = serde_json::json!({"audioStreams":[{"mimeType":"audio/webm","codec":"opus","bitrate":200000,"url":"https://example.org/opus"},{"mimeType":"audio/mp4","codec":"mp4a.40.2","bitrate":128000,"url":"https://example.org/aac"}]});
        assert_eq!(select_audio(&data), Some("https://example.org/aac"));
    }
}

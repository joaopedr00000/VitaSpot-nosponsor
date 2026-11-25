//! Public Audius catalog and independent local playback for Spotify Free users.
//! No Spotify credentials are sent to Audius. Only public playable tracks are used.
use std::{fs::File, io::Write, sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU32, Ordering}}, time::Duration};
use librespot_playback::{convert::Converter, decoder::{AudioDecoder, AudioPacket, SymphoniaDecoder}};
use serde_json::Value;
use crate::{abi::*, model::{Item, Page, Section}, state::Backend, util::{copy_str, now_ms_inner}, webapi::ApiError};

const API: &str = "https://api.audius.co/v1";
const MAX_AUDIO: usize = 32 * 1024 * 1024;
const MAX_JSON: usize = 2 * 1024 * 1024;
const NO_SEEK: u32 = u32::MAX;

fn client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder().https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .connect_timeout(Duration::from_secs(12))
        .timeout(Duration::from_secs(90))
        .user_agent("VitaSpot-Audius/0.1")
        .build().map_err(|_| ApiError::network("Audius connection failed."))
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}
fn id_from_uri(uri: &str) -> Option<&str> {
    uri.strip_prefix("audius:track:").filter(|id| valid_id(id))
}
async fn json(path: &str, query: &[(&str, &str)]) -> Result<Value, ApiError> {
    let mut response = client()?.get(format!("{API}{path}"))
        .query(query).send().await.map_err(|_| ApiError::network("Audius connection failed."))?;
    if response.status().as_u16() == 429 {
        return Err(ApiError::rate_limited("Audius rate limit reached."));
    }
    if !response.status().is_success() {
        return Err(ApiError::unavailable("Audius content unavailable."));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| ApiError::network("Audius connection failed."))? {
        if bytes.len().saturating_add(chunk.len()) > MAX_JSON {
            return Err(ApiError::unavailable("Audius response too large."));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| ApiError::unavailable("Invalid Audius response."))
}
fn item(v: &Value) -> Option<Item> {
    let id = v["id"].as_str().filter(|id| valid_id(id))?;
    let title = v["title"].as_str()?;
    let artist = v["user"]["name"].as_str().unwrap_or("Audius");
    let gated = v["is_stream_gated"].as_bool().unwrap_or(false)
        || v["is_streamable"].as_bool() == Some(false)
        || v["access"]["stream"].as_bool() == Some(false)
        || v["is_available"].as_bool() == Some(false);
    // The stream endpoint independently enforces any creator access restrictions.
    if gated { return None; }
    Some(Item {
        kind: VAM_KIND_TRACK, uri: format!("audius:track:{id}"), title: title.to_string(),
        subtitle: format!("{artist} · Audius"),
        image_url: v["artwork"]["150x150"].as_str()
            .filter(|url| url.starts_with("https://")).unwrap_or("").to_string(),
        duration_ms: v["duration"].as_u64().unwrap_or(0).saturating_mul(1000).min(u32::MAX as u64) as u32,
        is_explicit: v["is_explicit"].as_bool().unwrap_or(false),
        is_playable: true, ..Default::default()
    })
}
pub async fn page(uri: &str, limit: u32) -> Result<Page, ApiError> {
    let size = limit.clamp(1, 30).to_string();
    let (kind, title, response) = if uri == "vam:home" {
        (VAM_KIND_HOME, "Audius", json("/tracks/trending", &[("limit", &size), ("app_name", "VitaSpot")]).await?)
    } else if let Some(query) = uri.strip_prefix("vam:search:") {
        (VAM_KIND_SEARCH, "Audius", json("/tracks/search", &[("query", query), ("limit", &size), ("app_name", "VitaSpot")]).await?)
    } else if uri.starts_with("vam:library") || uri == "vam:queue" {
        return Ok(Page::new(if uri == "vam:queue" { VAM_KIND_QUEUE } else { VAM_KIND_LIBRARY }, uri));
    } else {
        return Err(ApiError::new(VAM_ERR_UNSUPPORTED, "Not available in Audius mode."));
    };
    let mut page = Page::new(kind, uri);
    page.title = title.into();
    let mut section = Section::list("Audius");
    if let Some(values) = response["data"].as_array() {
        for value in values.iter().take(limit.clamp(1, 30) as usize) {
            if let Some(track) = item(value) { section.push(track); }
        }
    }
    section.finish();
    page.sections.push(section);
    Ok(page)
}
async fn track(id: &str) -> Result<Item, ApiError> {
    let v = json(&format!("/tracks/{id}"), &[("app_name", "VitaSpot")]).await?;
    item(&v["data"]).ok_or_else(|| ApiError::unavailable("Audius content unavailable."))
}

#[derive(Default)]
struct Control {
    cancelled: AtomicBool,
    paused: AtomicBool,
    seek: AtomicU32,
    publish: Mutex<()>,
}
impl Control {
    fn cancelled(&self) -> bool { self.cancelled.load(Ordering::Acquire) }
    fn update(&self, backend: &Backend, f: impl FnOnce(&mut vam_player_state)) {
        let _guard = self.publish.lock().unwrap_or_else(|e| e.into_inner());
        if !self.cancelled() { backend.set_player(f); }
    }
}
#[derive(Default)]
pub struct LocalPlayer {
    current: Option<Arc<Control>>,
    last_uri: String,
    pub catalog: Arc<Mutex<Vec<Item>>>,
    queue: Vec<Item>,
    index: usize,
    generation: u64,
    // Prevent two Audius workers opening the same Vita audio port concurrently.
    output: Arc<Mutex<()>>,
}
impl LocalPlayer {
    pub fn stop(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(control) = self.current.take() {
            let _guard = control.publish.lock().unwrap_or_else(|e| e.into_inner());
            control.cancelled.store(true, Ordering::Release);
        }
    }
    pub fn play(&mut self, backend: Arc<Backend>, uri: &str) {
        let catalog = self.catalog.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(index) = catalog.iter().position(|item| item.uri == uri) {
            self.queue = catalog;
            self.index = index;
        } else {
            self.queue.clear();
            self.index = 0;
        }
        self.start(backend, uri);
    }
    fn start(&mut self, backend: Arc<Backend>, uri: &str) {
        let Some(id) = id_from_uri(uri).map(str::to_owned) else {
            backend.emit_error("Choose a track from Audius.");
            return;
        };
        self.stop();
        self.last_uri = uri.to_string();
        let generation = self.generation;
        let control = Arc::new(Control { seek: AtomicU32::new(NO_SEEK), ..Default::default() });
        self.current = Some(control.clone());
        let output = self.output.clone();
        backend.set_player(|state| {
            state.playback = VAM_PLAYBACK_BUFFERING;
            state.has_track = 0;
            state.is_active_device = 1;
            state.position_ms = 0;
            state.position_timestamp_ms = now_ms_inner();
            state.seq = state.seq.wrapping_add(1);
        });
        log::info!("Audius playback requested: {id}");
        tokio::spawn(async move {
            let result = async {
                let track = track(&id).await?;
                control.update(&backend, |s| {
                    s.track = track.to_c(); s.has_track = 1;
                    copy_str(&mut s.context_title, "Audius");
                    s.context_uri.fill(0);
                    s.seq = s.seq.wrapping_add(1);
                });
                if control.cancelled() { return Ok(()); }
                let file = download(&backend, &id, &control).await?;
                if control.cancelled() { return Ok(()); }
                let b = backend.clone();
                let c = control.clone();
                tokio::task::spawn_blocking(move || decode(&b, &c, &output, file))
                    .await.map_err(|_| ApiError::unavailable("Audius audio worker failed."))?
            }.await;
            if !control.cancelled() {
                let completed = result.is_ok();
                if let Err(error) = result {
                    log::warn!("Audius playback failed: {}", error.message);
                    backend.emit_error(&error.message);
                }
                control.update(&backend, |s| {
                    s.playback = VAM_PLAYBACK_STOPPED;
                    s.position_ms = 0; s.position_timestamp_ms = now_ms_inner();
                    s.seq = s.seq.wrapping_add(1);
                });
                log::logger().flush();
                if completed {
                    backend.send_command(crate::engine::Command::AudiusEnded(generation));
                }
            }
        });
    }
    pub fn ended(&mut self, backend: Arc<Backend>, generation: u64) {
        if generation == self.generation && self.index + 1 < self.queue.len() {
            self.skip(backend, true);
        }
    }
    pub fn skip(&mut self, backend: Arc<Backend>, forward: bool) {
        let index = if forward { self.index.checked_add(1) } else { self.index.checked_sub(1) };
        if let Some(index) = index.filter(|i| *i < self.queue.len()) {
            self.index = index;
            let uri = self.queue[index].uri.clone();
            self.start(backend, &uri);
        } else {
            backend.emit_toast("No more Audius tracks.");
        }
    }
    pub fn queue_page(&self) -> Page {
        let mut page = Page::new(VAM_KIND_QUEUE, "vam:queue");
        page.title = "Queue".into();
        let mut section = Section::list("Audius");
        for item in self.queue.iter().skip(self.index.saturating_add(1)) {
            section.push(item.clone());
        }
        section.finish();
        page.sections.push(section);
        page
    }
    pub fn add_to_queue(&mut self, backend: &Backend, uri: &str) {
        let catalog = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(item) = catalog.iter().find(|item| item.uri == uri) {
            // A single-track start may not have a catalog context.
            if self.queue.is_empty() {
                self.queue.push(Item { uri: self.last_uri.clone(), ..Default::default() });
                self.index = 0;
            }
            if self.queue.len() < 100 {
                self.queue.push(item.clone());
                backend.set_player(|s| s.seq = s.seq.wrapping_add(1));
                backend.emit_toast("Added to queue");
            }
        } else {
            backend.emit_toast("Not available in Audius mode.");
        }
    }
    pub fn reset(&mut self) {
        self.stop();
        self.last_uri.clear();
        self.queue.clear();
        self.index = 0;
        self.catalog.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
    pub fn pause(&self) {
        if let Some(c) = &self.current { c.paused.store(true, Ordering::Release); }
    }
    pub fn resume(&mut self, backend: Arc<Backend>) {
        if backend.player_snapshot().playback == VAM_PLAYBACK_STOPPED {
            let uri = self.last_uri.clone();
            if !uri.is_empty() { self.start(backend, &uri); }
        } else if let Some(c) = &self.current {
            c.paused.store(false, Ordering::Release);
        }
    }
    pub fn seek(&self, ms: u32) {
        if let Some(c) = &self.current { c.seek.store(ms, Ordering::Release); }
    }
}

struct Temporary(String);
impl Drop for Temporary {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}
async fn download(backend: &Backend, id: &str, control: &Control) -> Result<Temporary, ApiError> {
    let mut response = client()?.get(format!("{API}/tracks/{id}/stream"))
        .query(&[("app_name", "VitaSpot")]).send().await
        .map_err(|_| ApiError::network("Audius connection failed."))?;
    if !response.status().is_success() {
        return Err(ApiError::unavailable("Audius content unavailable."));
    }
    let content_type = response.headers().get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok()).unwrap_or("");
    if !content_type.starts_with("audio/") && !content_type.starts_with("application/octet-stream") {
        return Err(ApiError::unavailable("Unsupported Audius audio format."));
    }
    let temp = Temporary(backend.config.path(&format!("tmp/audius-{}-{}.mp3", now_ms_inner(), backend.next_request_id())));
    let mut file = File::create(&temp.0).map_err(|_| ApiError::unavailable("Audius temporary file failed."))?;
    let mut size = 0usize;
    while let Some(chunk) = response.chunk().await.map_err(|_| ApiError::network("Audius connection failed."))? {
        if control.cancelled() { break; }
        size = size.saturating_add(chunk.len());
        if size > MAX_AUDIO { return Err(ApiError::unavailable("Audius track too large.")); }
        file.write_all(&chunk).map_err(|_| ApiError::unavailable("Audius temporary file failed."))?;
    }
    drop(file);
    if size == 0 && !control.cancelled() { return Err(ApiError::unavailable("Audius content unavailable.")); }
    log::info!("Audius received {size} audio bytes");
    Ok(temp)
}

pub async fn image(url: &str) -> Result<Vec<u8>, ApiError> {
    let mut response = client()?.get(url).send().await.map_err(|_| ApiError::network("Audius connection failed."))?;
    if !response.status().is_success() { return Err(ApiError::unavailable("Audius content unavailable.")); }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| ApiError::network("Audius connection failed."))? {
        if bytes.len().saturating_add(chunk.len()) > MAX_JSON {
            return Err(ApiError::unavailable("Audius response too large."));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn decode(backend: &Backend, control: &Control, output: &Mutex<()>, file: Temporary) -> Result<(), ApiError> {
    crate::platform::lower_current_thread_priority();
    let _exclusive = output.lock().unwrap_or_else(|e| e.into_inner());
    if control.cancelled() { return Ok(()); }
    let err = |_| ApiError::unavailable("Unsupported Audius audio format.");
    let input = File::open(&file.0).map_err(|_| ApiError::unavailable("Audius temporary file failed."))?;
    let mut decoder = SymphoniaDecoder::new_mp3_file(input).map_err(err)?;
    let mut sink = crate::platform::make_sink();
    let mut converter = Converter::new(None);
    let sink_error = |_| ApiError::unavailable("Audius audio output failed.");
    sink.start().map_err(sink_error)?;
    let result = (|| {
        let mut position = 0;
        let mut playing = false;
        let mut paused = false;
        let mut last_update = 0;
        while !control.cancelled() {
            if control.paused.load(Ordering::Acquire) {
                if !paused {
                    sink.flush().map_err(sink_error)?;
                    control.update(backend, |s| { s.playback = VAM_PLAYBACK_PAUSED; s.position_ms = position; s.position_timestamp_ms = now_ms_inner(); });
                    paused = true; playing = false;
                }
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            let seek = control.seek.swap(NO_SEEK, Ordering::AcqRel);
            if seek != NO_SEEK {
                position = decoder.seek(seek).map_err(err)?;
                sink.flush().map_err(sink_error)?;
                playing = false;
            }
            paused = false;
            let Some((pos, mut packet)) = decoder.next_packet().map_err(err)? else {
                // Allow the native output ring to drain before closing its port.
                std::thread::sleep(Duration::from_millis(1100));
                break;
            };
            position = pos.position_ms;
            // Vita uses hardware volume; the host sink uses local software scaling.
            if crate::platform::system_volume_step().is_none() {
                if let AudioPacket::Samples(samples) = &mut packet {
                    let gain = backend.player_snapshot().volume as f64 / 65535.0;
                    for sample in samples { *sample *= gain; }
                }
            }
            if control.cancelled() { break; }
            sink.write(packet, &mut converter).map_err(sink_error)?;
            let now = now_ms_inner();
            if !playing || now.saturating_sub(last_update) >= 250 {
                if !playing { log::info!("Audius PCM submitted to audio sink"); }
                playing = true; last_update = now;
                control.update(backend, |s| {
                    s.playback = VAM_PLAYBACK_PLAYING;
                    s.position_ms = position;
                    s.position_timestamp_ms = now;
                });
            }
        }
        Ok(())
    })();
    let _ = sink.stop();
    result
}

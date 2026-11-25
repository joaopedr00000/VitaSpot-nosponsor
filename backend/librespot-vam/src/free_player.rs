//! Free-account playback only. The Spotify library, metadata, and saved
//! tracks remain untouched; the alternate provider only resolves audio.
use std::{fs::File, sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU32, Ordering}}, time::Duration};
use librespot_playback::{convert::Converter, decoder::AudioPacket};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error as DecodeError,
    formats::{FormatOptions, SeekMode, SeekTo},
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions, probe::Hint, units::Time,
};
use crate::{abi::*, free_source::{self, Result, TempAudio}, metadata, model::Item, pages,
    shared::{Shared, PlayerMeta}, util::{copy_str, now_ms_inner}};

const NO_SEEK: u32 = u32::MAX;
const MAX_QUEUE: usize = 200;
#[derive(Default)]
struct Control {
    stop: AtomicBool,
    paused: AtomicBool,
    seek: AtomicU32,
    publish: Mutex<()>,
}
impl Control {
    fn stopped(&self) -> bool { self.stop.load(Ordering::Acquire) }
    fn update(&self, shared: &Shared, f: impl FnOnce(&mut vam_player_state)) {
        let _guard = self.publish.lock().unwrap_or_else(|e| e.into_inner());
        if !self.stopped() { shared.backend.set_player(f); }
    }
}
#[derive(Default)]
pub struct FreePlayer {
    control: Option<Arc<Control>>,
    task: Option<tokio::task::JoinHandle<()>>,
    output: Arc<Mutex<()>>,
    generation: u64,
    queue: Arc<Mutex<Vec<Item>>>,
    requested: Arc<Mutex<String>>,
}
impl FreePlayer {
    pub fn stop(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(c) = self.control.take() {
            let _guard = c.publish.lock().unwrap_or_else(|e| e.into_inner());
            c.stop.store(true, Ordering::Release);
        }
        if let Some(task) = self.task.take() { task.abort(); }
    }
    pub fn reset(&mut self) {
        self.stop();
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.requested.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
    pub fn play(&mut self, shared: Arc<Shared>, context: String, track: String, index: u32) {
        self.start(shared, context, track, index, true);
    }
    fn start(&mut self, shared: Arc<Shared>, context: String, track: String, index: u32, replace_queue: bool) {
        self.stop();
        let control = Arc::new(Control { seek: AtomicU32::new(NO_SEEK), ..Default::default() });
        self.control = Some(control.clone());
        let output = self.output.clone();
        let generation = self.generation;
        let queue = self.queue.clone();
        let requested = self.requested.clone();
        *requested.lock().unwrap_or_else(|e| e.into_inner()) = track.clone();
        shared.backend.set_player(|s| {
            s.playback = VAM_PLAYBACK_BUFFERING;
            s.has_track = 0; s.is_active_device = 1;
            s.position_ms = 0; s.position_timestamp_ms = now_ms_inner();
            s.seq = s.seq.wrapping_add(1);
        });
        self.task = Some(tokio::spawn(async move {
            let result: Result<()> = async {
                let mut items = if replace_queue && !context.is_empty() && !context.starts_with("spotify:track:") {
                    match pages::build_page(&shared, &context, 0, MAX_QUEUE as u32).await {
                        Ok(page) => page.sections.into_iter().flat_map(|s| s.items)
                            .filter(|i| i.kind == VAM_KIND_TRACK && i.uri.starts_with("spotify:track:"))
                            .take(MAX_QUEUE).collect::<Vec<_>>(),
                        Err(_) => Vec::new(),
                    }
                } else if !replace_queue {
                    queue.lock().unwrap_or_else(|e| e.into_inner()).clone()
                } else { Vec::new() };
                let uri = if track.starts_with("spotify:track:") { track.clone() }
                    else if track.is_empty() && context.starts_with("spotify:track:") { context.clone() }
                    else if track.is_empty() { items.get(index as usize).map(|i| i.uri.clone()).ok_or(free_source::NO_MATCH)? }
                    else { return Err(free_source::NO_MATCH); };
                let selected = if let Some(item) = items.iter().find(|i| i.uri == uri) { item.clone() }
                    else { metadata::resolve_playables(&shared.web, &shared.meta, &[uri.clone()]).await
                        .into_iter().next().ok_or(free_source::NO_MATCH)? };
                if selected.kind != VAM_KIND_TRACK || !selected.is_playable || selected.duration_ms == 0 {
                    return Err(free_source::NO_MATCH);
                }
                if !items.iter().any(|i| i.uri == uri) { items.insert(0, selected.clone()); }
                {
                    let _guard = control.publish.lock().unwrap_or_else(|e| e.into_inner());
                    if control.stopped() { return Ok(()); }
                    *queue.lock().unwrap_or_else(|e| e.into_inner()) = items.clone();
                    *requested.lock().unwrap_or_else(|e| e.into_inner()) = uri.clone();
                    let at = items.iter().position(|i| i.uri == uri).unwrap_or(0);
                    shared.update_player(|meta| {
                        *meta = PlayerMeta {
                            track_uri: uri.clone(), track: Some(selected.clone()),
                            context_uri: context.clone(), context_title: "Spotify".into(), is_active: true,
                            prev_tracks: items[..at].iter().map(|i| i.uri.clone()).collect(),
                            next_tracks: items[at+1..].iter().map(|i| i.uri.clone()).collect(),
                        };
                    });
                    shared.backend.set_player(|s| {
                        s.track = selected.to_c(); s.has_track = 1;
                        copy_str(&mut s.context_uri, &context);
                        copy_str(&mut s.context_title, "Spotify");
                        s.seq = s.seq.wrapping_add(1);
                    });
                }
                log::info!("Free playback: Spotify metadata selected, resolving alternate audio");
                let audio = tokio::time::timeout(Duration::from_secs(90), free_source::resolve(&shared.backend, &selected)).await
                    .map_err(|_| { log::warn!("Free source: overall resolution timeout"); free_source::NETWORK })??;
                if control.stopped() { return Ok(()); }
                let s = shared.clone();
                let c = control.clone();
                tokio::task::spawn_blocking(move || decode(&s, &c, &output, audio, false))
                    .await.map_err(|_| free_source::AUDIO_ERROR)?
            }.await;
            if !control.stopped() {
                let completed = result.is_ok();
                if let Err(error) = result {
                    log::warn!("Free playback failed: {error}");
                    shared.backend.emit_error(error);
                }
                control.update(&shared, |s| {
                    s.playback = VAM_PLAYBACK_STOPPED; s.position_ms = 0;
                    s.position_timestamp_ms = now_ms_inner(); s.seq = s.seq.wrapping_add(1);
                });
                log::logger().flush();
                if completed { shared.backend.send_command(crate::engine::Command::FreeEnded(generation)); }
            }
        }));
    }
    /// Local AAC test; no network requests or changes to Spotify metadata.
    pub fn selftest(&mut self, shared: Arc<Shared>) {
        if shared.backend.player_snapshot().playback != VAM_PLAYBACK_STOPPED
            || self.task.as_ref().is_some_and(|t| !t.is_finished()) {
            shared.backend.emit_toast("Stop playback before testing audio.");
            return;
        }
        self.stop();
        let c = Arc::new(Control { seek: AtomicU32::new(NO_SEEK), ..Default::default() });
        self.control = Some(c.clone());
        let output = self.output.clone();
        self.task = Some(tokio::spawn(async move {
            shared.backend.emit_toast("Playing local AAC test tone.");
            log::info!("SELFTEST AAC: starting local 48kHz mono fixture");
            let file = TempAudio(std::path::PathBuf::from(shared.backend.config.path(&format!("tmp/free-audio-test-{}.m4a", shared.backend.next_request_id()))));
            let result = if std::fs::write(&file.0, include_bytes!("../tests/fixtures/free-tone.m4a")).is_ok() {
                let s = shared.clone();
                let ctl = c.clone();
                tokio::task::spawn_blocking(move || decode(&s, &ctl, &output, file, true))
                    .await.unwrap_or(Err(free_source::AUDIO_ERROR))
            } else { Err(free_source::FILE_ERROR) };
            if !c.stopped() {
                match result {
                    Ok(()) => { log::info!("SELFTEST AAC: PCM submitted; verify tone was audible"); shared.backend.emit_toast("Audio test finished. Did you hear the tone?"); }
                    Err(e) => { log::warn!("SELFTEST AAC: failed: {e}"); shared.backend.emit_error(e); }
                }
            }
            log::logger().flush();
        }));
    }
    pub fn pause(&self) {
        if let Some(c) = &self.control { c.paused.store(true, Ordering::Release); }
    }
    pub fn resume(&mut self, shared: Arc<Shared>) {
        if shared.backend.player_snapshot().playback == VAM_PLAYBACK_STOPPED {
            let uri = self.requested.lock().unwrap_or_else(|e| e.into_inner()).clone();
            if !uri.is_empty() { self.start(shared, String::new(), uri, 0, false); }
        } else if let Some(c) = &self.control { c.paused.store(false, Ordering::Release); }
    }
    pub fn seek(&self, position: u32) {
        if let Some(c) = &self.control { c.seek.store(position, Ordering::Release); }
    }
    pub fn skip(&mut self, shared: Arc<Shared>, forward: bool) {
        let current = self.requested.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let next = {
            let queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
            queue.iter().position(|i| i.uri == current).and_then(|at| {
                let index = if forward { at.checked_add(1) } else { at.checked_sub(1) }?;
                queue.get(index).map(|i| i.uri.clone())
            })
        };
        if let Some(uri) = next { self.start(shared, String::new(), uri, 0, false); }
        else { shared.backend.emit_toast("No more tracks in the Free queue."); }
    }
    pub fn ended(&mut self, shared: Arc<Shared>, generation: u64) {
        if generation == self.generation {
            let current = self.requested.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let has_next = { let q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
                q.iter().position(|i| i.uri == current).is_some_and(|at| at+1 < q.len()) };
            if has_next { self.skip(shared, true); }
        }
    }
    pub fn add(&self, shared: &Shared, uri: &str) {
        if let Some(item) = shared.meta.get(uri) {
            let mut q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
            if q.len() < MAX_QUEUE {
                q.push(item);
                shared.update_player(|m| m.next_tracks.push(uri.to_owned()));
                shared.backend.emit_toast("Added to queue");
                return;
            }
        }
        shared.backend.emit_toast("Free queue item unavailable.");
    }
}

// Linear stereo resampling with continuous phase across decoded packets.
// Native sink runs at 44.1kHz; AAC commonly arrives at 44.1 or 48kHz.
#[derive(Default)]
struct Resampler { prev: Option<[f64; 2]>, phase: f64 }
impl Resampler {
    fn convert(&mut self, input: &[f64], channels: usize, rate: u32) -> Vec<f64> {
        let step = rate as f64 / 44100.0;
        let mut out = Vec::with_capacity(input.len()*2+8);
        for frame in input.chunks_exact(channels) {
            let current = [frame[0], if channels == 2 { frame[1] } else { frame[0] }];
            if let Some(previous) = self.prev {
                while self.phase <= 1.0 {
                    for ch in 0..2 { out.push(previous[ch]+(current[ch]-previous[ch])*self.phase); }
                    self.phase += step;
                }
                self.phase -= 1.0;
            } else { out.extend_from_slice(&current); self.phase = step; }
            self.prev = Some(current);
        }
        out
    }
}
fn decode(shared: &Shared, control: &Control, output: &Mutex<()>, audio: TempAudio, diagnostic: bool) -> Result<()> {
    crate::platform::lower_current_thread_priority();
    let _exclusive = output.lock().unwrap_or_else(|e| e.into_inner());
    if control.stopped() { return Ok(()); }
    let input = File::open(&audio.0).map_err(|_| free_source::FILE_ERROR)?;
    let stream = MediaSourceStream::new(Box::new(input), MediaSourceStreamOptions::default());
    let mut hint = Hint::new(); hint.with_extension("m4a");
    let mut format = symphonia::default::get_probe().format(&hint, stream, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| { log::warn!("Free M4A probe: {e}"); free_source::AUDIO_ERROR })?.format;
    let track = format.default_track().ok_or(free_source::AUDIO_ERROR)?;
    let id = track.id;
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| { log::warn!("Free AAC decoder: {e}"); free_source::AUDIO_ERROR })?;
    log::info!("Free audio: container and decoder initialized");
    let mut sink = crate::platform::make_sink();
    let mut converter = Converter::new(None);
    if let Err(e) = sink.start() { log::warn!("Free sink start: {e}"); let _ = sink.stop(); return Err(free_source::OUTPUT_ERROR); }
    let result = (|| {
        let mut resample = Resampler::default();
        let mut frames: u64 = 0;
        let mut last_update = 0;
        let mut playing = false;
        let mut paused = false;
        while !control.stopped() {
            if !diagnostic && control.paused.load(Ordering::Acquire) {
                if !paused {
                    sink.flush().map_err(|_| free_source::OUTPUT_ERROR)?;
                    control.update(shared, |s| { s.playback = VAM_PLAYBACK_PAUSED; s.position_ms = (frames*1000/44100) as u32; s.position_timestamp_ms = now_ms_inner(); });
                    paused = true; playing = false;
                }
                std::thread::sleep(Duration::from_millis(50)); continue;
            }
            let seek = if diagnostic { NO_SEEK } else { control.seek.swap(NO_SEEK, Ordering::AcqRel) };
            if seek != NO_SEEK {
                let time = Time::new((seek/1000) as u64, (seek%1000) as f64/1000.0);
                format.seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(id) })
                    .map_err(|_| free_source::AUDIO_ERROR)?;
                decoder.reset(); resample = Resampler::default();
                frames = seek as u64 * 44100 / 1000;
                sink.flush().map_err(|_| free_source::OUTPUT_ERROR)?;
                playing = false;
            }
            paused = false;
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(DecodeError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    if frames == 0 { return Err(free_source::AUDIO_ERROR); }
                    std::thread::sleep(Duration::from_millis(1100)); break;
                }
                Err(e) => { log::warn!("Free packet read: {e}"); return Err(free_source::AUDIO_ERROR); }
            };
            if packet.track_id() != id { continue; }
            let decoded = decoder.decode(&packet).map_err(|e| { log::warn!("Free AAC packet: {e}"); free_source::AUDIO_ERROR })?;
            let spec = *decoded.spec();
            let channels = spec.channels.count();
            if !(1..=2).contains(&channels) || !(8000..=96000).contains(&spec.rate) { return Err(free_source::NO_FORMAT); }
            // Allocate for each packet's actual shape; no stale buffer capacity.
            let mut buf = SampleBuffer::<f64>::new(decoded.capacity() as u64, spec);
            buf.copy_interleaved_ref(decoded);
            let mut pcm = resample.convert(buf.samples(), channels, spec.rate);
            if pcm.is_empty() { continue; }
            if crate::platform::system_volume_step().is_none() {
                let gain = shared.backend.player_snapshot().volume as f64 / 65535.0;
                for sample in &mut pcm { *sample *= gain; }
            }
            if control.stopped() { break; }
            let count = pcm.len()/2;
            sink.write(AudioPacket::Samples(pcm), &mut converter).map_err(|_| free_source::OUTPUT_ERROR)?;
            frames += count as u64;
            let now = now_ms_inner();
            if !playing || now.saturating_sub(last_update) >= 250 {
                if !playing { log::info!("Free source: decoded AAC PCM submitted to audio sink ({} Hz, {channels} channels)", spec.rate); }
                playing = true; last_update = now;
                if !diagnostic { control.update(shared, |s| {
                    s.playback = VAM_PLAYBACK_PLAYING; s.position_ms = ((frames*1000/44100) as u32).min(s.track.duration_ms);
                    s.position_timestamp_ms = now; s.seq = s.seq.wrapping_add(1);
                }); }
            }
        }
        Ok(())
    })();
    let _ = sink.stop();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn resamples_48k_mono_to_441_stereo() {
        let mut r = Resampler::default();
        let samples = r.convert(&vec![0.25; 48000], 1, 48000);
        assert!((samples.len() as i64 - 88200).abs() <= 4);
        assert!(samples.iter().all(|v| (*v-0.25).abs()<1e-6));
    }
}

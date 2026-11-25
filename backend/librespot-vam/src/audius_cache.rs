//! Bounded, private streaming cache. Five complete/partial tracks at most,
//! with only the last selected, complete track surviving a new process.
use std::{collections::VecDeque, fs, io, path::{Path, PathBuf}};
use crate::{model::Item, state::Backend};

const LIMIT: usize = 5;
const MAX_BYTES: u64 = 32 * 1024 * 1024;

pub struct Cache {
    dir: PathBuf,
    order: VecDeque<String>,
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}
fn item_id(item: &Item) -> Option<&str> {
    item.uri.strip_prefix("audius:track:").filter(|id| valid_id(id))
}
fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
fn usable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0 && m.len() <= MAX_BYTES)
}
impl Cache {
    pub fn new(backend: &Backend) -> Self {
        Self::open(PathBuf::from(backend.config.path("cache/audius")))
    }
    fn open(dir: PathBuf) -> Self {
        let mut cache = Self { dir, order: VecDeque::new() };
        if let Err(e) = cache.prune_startup() {
            log::warn!("Audius cache startup cleanup failed: {e}");
        }
        cache
    }
    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("track-{id}.mp3"))
    }
    pub fn partial(&self, id: &str) -> PathBuf {
        self.dir.join(format!("track-{id}.part"))
    }
    fn marker(&self) -> PathBuf { self.dir.join("current.json") }
    pub fn clear_selection(backend: &Backend) {
        let path = PathBuf::from(backend.config.path("cache/audius/current.json"));
        if let Err(e) = remove(&path) {
            log::warn!("Audius cache selection cleanup failed: {e}");
        }
    }
    pub fn restored(&self) -> Option<Item> {
        let marker = self.marker();
        if fs::metadata(&marker).ok()?.len() > 16384 { return None; }
        let item: Item = serde_json::from_slice(&fs::read(marker).ok()?).ok()?;
        let id = item_id(&item)?;
        if item.kind == crate::abi::VAM_KIND_TRACK && item.is_playable && usable(&self.path(id)) {
            Some(item)
        } else { None }
    }
    fn prune_startup(&mut self) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let restored = self.restored();
        let keep = restored.as_ref().and_then(item_id);
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue; };
            if name == "current.tmp" || (name.starts_with("track-") &&
                (name.ends_with(".mp3") || name.ends_with(".part")) &&
                !keep.is_some_and(|id| name == format!("track-{id}.mp3"))) {
                remove(&entry.path())?;
            }
        }
        if let Some(id) = keep { self.order.push_back(id.to_string()); }
        else { remove(&self.marker())?; }
        log::info!("Audius cache startup: {} track retained", self.order.len());
        Ok(())
    }
    fn touch(&mut self, id: &str) {
        self.order.retain(|s| s != id);
        self.order.push_back(id.to_string());
    }
    pub fn hit(&mut self, id: &str) -> Option<PathBuf> {
        if !valid_id(id) { return None; }
        let path = self.path(id);
        if usable(&path) {
            self.touch(id);
            log::info!("Audius cache hit: {id}");
            Some(path)
        } else { None }
    }
    /// Reserve a slot before any new bytes are written. Failed deletions
    /// prevent a new download rather than exceeding the five-track cap.
    pub fn reserve(&mut self, id: &str) -> io::Result<PathBuf> {
        if !valid_id(id) { return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid Audius ID")); }
        fs::create_dir_all(&self.dir)?;
        // Reconcile disk files after interrupted downloads or cleanup errors.
        let mut present = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue; };
            if let Some(other) = name.strip_prefix("track-").and_then(|s| s.strip_suffix(".part")) {
                if valid_id(other) { remove(&entry.path())?; }
            } else if let Some(other) = name.strip_prefix("track-").and_then(|s| s.strip_suffix(".mp3")) {
                if valid_id(other) {
                    if other == id || !usable(&entry.path()) { remove(&entry.path())?; }
                    else { present.push(other.to_string()); }
                }
            }
        }
        self.order.retain(|s| present.contains(s));
        for other in present {
            if !self.order.contains(&other) { self.order.push_front(other); }
        }
        while self.order.len() >= LIMIT {
            let old = self.order.front().cloned().unwrap();
            remove(&self.path(&old))?;
            self.order.pop_front();
            log::info!("Audius cache evicted: {old}");
        }
        Ok(self.partial(id))
    }
    pub fn commit(&mut self, id: &str, partial: &Path) -> io::Result<PathBuf> {
        let path = self.path(id);
        if !usable(partial) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid cached audio"));
        }
        fs::rename(partial, &path)?;
        self.touch(id);
        log::info!("Audius cache stored: {id} ({}/{LIMIT})", self.order.len());
        Ok(path)
    }
    pub fn mark_current(&self, item: &Item) -> io::Result<()> {
        let id = item_id(item).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid Audius item"))?;
        if !usable(&self.path(id)) { return Err(io::Error::new(io::ErrorKind::NotFound, "track not cached")); }
        let bytes = serde_json::to_vec(item).map_err(io::Error::other)?;
        let tmp = self.dir.join("current.tmp");
        fs::write(&tmp, bytes)?;
        // Some filesystems do not support rename over an existing destination.
        if let Err(e) = fs::rename(&tmp, self.marker()) {
            if !self.marker().exists() { return Err(e); }
            remove(&self.marker())?;
            fs::rename(&tmp, self.marker())?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_tracks_and_restart_retains_only_current() {
        let dir = std::env::temp_dir().join(format!("vam-audius-cache-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut cache = Cache::open(dir.clone());
        for i in 1..=6 {
            let id = format!("T{i}");
            let part = cache.reserve(&id).unwrap();
            fs::write(&part, b"test").unwrap();
            cache.commit(&id, &part).unwrap();
        }
        assert!(!cache.path("T1").exists());
        assert_eq!(cache.order.len(), 5);
        assert!(cache.hit("T2").is_some());
        let part = cache.reserve("T7").unwrap();
        fs::write(&part, b"test").unwrap();
        cache.commit("T7", &part).unwrap();
        assert!(!cache.path("T3").exists());
        let item = Item { kind: crate::abi::VAM_KIND_TRACK, uri: "audius:track:T7".into(),
            title: "Test".into(), is_playable: true, ..Default::default() };
        cache.mark_current(&item).unwrap();
        fs::write(cache.partial("T8"), b"incomplete").unwrap();
        let cache = Cache::open(dir.clone());
        assert_eq!(cache.order.len(), 1);
        assert_eq!(cache.restored().unwrap().uri, item.uri);
        assert!(!cache.path("T2").exists());
        assert!(!cache.partial("T8").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}

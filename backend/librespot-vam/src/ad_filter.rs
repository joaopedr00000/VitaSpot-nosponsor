//! Small, local ad filter. No wildcard DNS changes, TLS interception,
//! forged account attributes or changes to Spotify playback authorization.
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;

static ENABLED: AtomicBool = AtomicBool::new(true);
static MUTED: AtomicBool = AtomicBool::new(false);

pub fn configure(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
    reset();
}
pub fn reset() {
    MUTED.store(false, Ordering::Relaxed);
}
pub fn muted() -> bool {
    MUTED.load(Ordering::Relaxed)
}
fn explicit_ad(uri: &str, metadata: &HashMap<String, String>) -> bool {
    uri.starts_with("spotify:ad:") ||
        (!uri.is_empty() && ["is_ad", "is_advertisement", "advertisement"]
            .iter().any(|key| metadata.get(*key).is_some_and(|v|
                v == "1" || v.eq_ignore_ascii_case("true"))))
}
pub fn update(uri: &str, metadata: &HashMap<String, String>, active: bool) {
    let mute = ENABLED.load(Ordering::Relaxed) && active && explicit_ad(uri, metadata);
    if MUTED.swap(mute, Ordering::Relaxed) != mute {
        log::info!("ad filter: local audio mute {}", if mute { "on" } else { "off" });
    }
}

// Only exact, dedicated advertising hosts in the image fetch path.
// Never block authentication, metadata, shared CDNs or audio storage.
fn ad_image_host(url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(url) else { return false; };
    let Some(host) = url.host_str() else { return false; };
    matches!(host.trim_end_matches('.'),
        "ads-fa.spotify.com" | "pubads.g.doubleclick.net" |
        "securepubads.g.doubleclick.net" | "pagead2.googlesyndication.com" |
        "tpc.googlesyndication.com" | "googleads.g.doubleclick.net")
}
pub fn block_image(url: &str) -> bool {
    ENABLED.load(Ordering::Relaxed) && ad_image_host(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_signals_only() {
        let mut meta = HashMap::new();
        meta.insert("title".into(), "Advertisement".into());
        assert!(!explicit_ad("spotify:track:123", &meta));
        assert!(explicit_ad("spotify:ad:123", &meta));
        meta.insert("is_ad".into(), "true".into());
        assert!(explicit_ad("spotify:track:123", &meta));
        assert!(!explicit_ad("", &meta));
        meta.insert("is_ad".into(), "false".into());
        assert!(!explicit_ad("spotify:episode:123", &meta));
    }
    #[test]
    fn only_dedicated_hosts() {
        assert!(ad_image_host("https://ads-fa.spotify.com/banner.png"));
        assert!(ad_image_host("https://securepubads.g.doubleclick.net/banner"));
        assert!(!ad_image_host("https://i.scdn.co/image/123"));
        assert!(!ad_image_host("https://accounts.spotify.com/authorize"));
        assert!(!ad_image_host("https://spclient.wg.spotify.com/metadata"));
        assert!(!ad_image_host("https://ads-fa.spotify.com.example.org/"));
        assert!(!ad_image_host("https://ads-fa.spotify.com@i.scdn.co/image"));
    }
}

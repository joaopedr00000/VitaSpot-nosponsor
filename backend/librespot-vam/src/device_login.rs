//! RFC 8628 pairing. No password, local callback, or external relay.
use std::sync::Arc;
use std::time::Duration;
use librespot_core::{authentication::Credentials, SessionConfig};
use serde::Deserialize;
use crate::{abi::*, state::Backend, util::copy_str};

const PAIR: &str = "https://spotify.com/pair";
const SCOPES: &str = "app-remote-control playlist-read playlist-read-collaborative playlist-read-private streaming user-follow-modify user-follow-read user-library-modify user-library-read user-read-currently-playing user-read-playback-position user-read-playback-state user-read-recently-played user-read-private user-top-read";

#[derive(Deserialize)]
struct Pairing {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    #[serde(default = "default_interval")]
    interval: u64,
}
fn default_interval() -> u64 { 5 }

pub async fn run(backend: Arc<Backend>) -> Result<Credentials, &'static str> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_| "Could not initialize pairing network.")?;
    // Keep the same client identity used by the existing login helper/session.
    let client_id = SessionConfig::default().client_id;
    let response = http.post("https://accounts.spotify.com/oauth2/device/authorize")
        .form(&[("client_id", client_id.as_str()), ("scope", SCOPES)])
        .send().await.map_err(|_| "Pairing network request failed.")?;
    if !response.status().is_success() {
        return Err("Spotify did not authorize device pairing. Try Connect or PC login.");
    }
    let data = response.bytes().await.map_err(|_| "Could not read pairing response.")?;
    let pair: Pairing = serde_json::from_slice(&data).map_err(|_| "Invalid pairing response.")?;
    if pair.verification_uri.trim_end_matches('/') != PAIR
        || pair.user_code.is_empty() || pair.user_code.len() >= 32
        || !pair.user_code.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || pair.device_code.is_empty() || pair.expires_in == 0 {
        return Err("Spotify returned an unsupported pairing code.");
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(pair.expires_in.min(3600));
    let mut interval = pair.interval.max(5);
    backend.set_auth(|auth| {
        auth.status = VAM_AUTH_LOGGED_OUT;
        copy_str(&mut auth.login_url, PAIR);
        copy_str(&mut auth.login_code, &pair.user_code);
        copy_str(&mut auth.message, "Scan the QR code and enter the pairing code.");
    });
    loop {
        let delay = Duration::from_secs(interval);
        if tokio::time::Instant::now() + delay >= deadline {
            return Err("Pairing code expired. Requesting a new code shortly.");
        }
        tokio::time::sleep(delay).await;
        let response = match http.post("https://accounts.spotify.com/api/token")
            .form(&[("client_id", client_id.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", pair.device_code.as_str())])
            .send().await {
                Ok(r) => r,
                Err(_) => { interval = interval.saturating_add(5).min(60); continue; }
            };
        let status = response.status();
        if status.as_u16() == 429 || status.is_server_error() {
            interval = interval.saturating_add(5).min(60);
            continue;
        }
        let data = response.bytes().await.map_err(|_| "Could not read pairing status.")?;
        let body: serde_json::Value = serde_json::from_slice(&data)
            .map_err(|_| "Invalid pairing status response.")?;
        if status.is_success() {
            let token = body["access_token"].as_str().filter(|s| !s.is_empty())
                .ok_or("Spotify did not return an access token.")?;
            return Ok(Credentials::with_access_token(token));
        }
        match body["error"].as_str() {
            Some("authorization_pending") => {},
            Some("slow_down") => interval = interval.saturating_add(5),
            Some("access_denied") => return Err("Pairing declined. Try again shortly."),
            Some("expired_token") => return Err("Pairing code expired. Requesting a new code shortly."),
            _ => return Err("Spotify rejected the pairing request. Try Connect or PC login."),
        }
    }
}

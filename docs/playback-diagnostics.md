# Playback compatibility and diagnostic build

The native Vita backend still uses librespot. Signing in does not establish entitlement to a track or permission to use the legacy Connect protocol. Upstream reports that legacy eSDK v1 access is restricted by Spotify and that affected accounts receive audio key error 0 1 across librespot versions. See https://github.com/librespot-org/librespot/issues/1649 and https://github.com/librespot-org/librespot.

## On-device test

1. Install the latest diagnostic VPK (0.0.9d or later) over the existing application. Confirm the version in Settings. Keep credentials and preferences.
2. Fully exit, start again, select a track and attempt playback once.
3. Send ux0:data/VitaSpot/vam-diagnostic.log (fallback: vam.log if the new file cannot be opened). The new file begins with "VitaSpot backend diagnostic revision 1". Do not send credentials.json or tokens.
4. If feasible, repeat with an authorized, working Premium account on the same Vita, same network and same track. Compare the logs; this is a control test, not a requirement to share credentials.
5. Try an available podcast episode separately, and note whether it actually produces sound. Podcast support varies by account and content.

Markers:
- "account authorized for login": authentication succeeded, not proof of audio authorization.
- "Audio authorization/key unavailable": no protected audio is passed to the decoder.
- "audio sink received decoded PCM": decoded samples have reached the native audio sink; not proof of audible output.
- "player confirmed playback": actual player event, distinct from a position correction.
- "playback buffering timeout": the waiting deadline expired.

## Official Soloist experiment (external host, not a native Vita backend)

Spotify Soloist provides official playback on supported Linux, macOS and Windows hosts. The ARMv7 build is a Linux executable, not a Vita executable; its OS ABI and PipeWire/PulseAudio dependencies cannot be satisfied merely by sharing a CPU instruction set. A native port is not established, and the official binary must not be redistributed.

Download from https://developer.spotify.com/documentation/soloist/reference/downloads-and-updates and obtain your own API key as documented by Spotify (Premium required to provision; Free and Premium users can subsequently connect). On a supported Linux host, with a valid key in the environment, run:

    ./soloist --device-name "VitaSpot external test" --api-key "$SOLOIST_API_KEY" --ws 127.0.0.1:9090

Use the official Spotify app to select the receiver on the same LAN, then test a Free account. Audio plays on the host. This is a separate test, not an integration with the Vita VPK. Keep the control API on loopback: it has no built-in client authentication or TLS. Never commit or share the API key. Builds expire 90 days from their build date.

References:
- https://developer.spotify.com/documentation/soloist
- https://developer.spotify.com/documentation/soloist/reference/command-line
- https://developer.spotify.com/documentation/soloist/concepts/overview
- https://developer.spotify.com/blog/2026-08-13-introducing-spotify-soloist

The diagnostic build does not restore server-side playback permissions. It does not ship Soloist or change its restrictions.

## Local Soloist API probe (Windows or Linux/macOS)

The repository includes [tools/soloist_probe.py](../tools/soloist_probe.py).
It uses the official `soloist ctl` client to query status, optionally request a
track, and sample playback four times. Python 3 is sufficient; no third-party
packages, Spotify secrets, or network listener are required by the probe itself.

Generate a personal **Soloist API Key** in https://developer.spotify.com/dashboard
and launch the official binary with the loopback WebSocket endpoint described above.
On Windows, from the extracted Soloist directory, use:

    .\soloist.exe --device-name "VitaSpot external test" --api-key $env:SOLOIST_API_KEY --ws 127.0.0.1:9090

Set the environment variable privately on your own computer. Do not send the key
to another person or commit it. Pair the receiver from the Spotify app using
the account being tested. In another terminal, run the probe with the path to
the official executable:

    python tools/soloist_probe.py --soloist "C:\path\to\soloist.exe"

For a direct track test, add `--track` followed by a Spotify track link.
If the Free account cannot select a specific track, use ordinary playback from
the official Spotify app and run the probe without that argument.

A successful command is not proof of audible sound. Confirm audio output on
the computer and compare with the Vita diagnostic log. Do not publish raw
account or session data. This probe does not stream audio to Vita.

A regular Spotify Web API Client ID/Secret is not a Soloist API key. The
Web API does not deliver full-track audio, and its playback control endpoint
requires Premium. Do not replace the working QR login with arbitrary developer
credentials or embed a client secret into the VPK. See
https://developer.spotify.com/documentation/web-api/reference/start-a-users-playback
and https://developer.spotify.com/documentation/soloist/reference/soloist-ctl.

## 0.0.15d experimental Free audio compatibility

The Free decoder probes actual media bytes and enables AAC-LC/M4A, MP3, Ogg Vorbis, FLAC and PCM/WAV. It logs the advertised codec and MIME type, retries supported candidates and rejects files that fail the decoder probe. Opus and HE-AAC are not supported by this build. No new external server has been verified. Premium playback and Spotify account handling are unchanged. The host CI validates synthetic audio for all five formats; audible output on Vita remains a hardware test. The local AAC self-test remains available. Current log names are `vam-audio-015.log` and `vam-ui-015.log`.

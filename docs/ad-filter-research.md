# Native ad filtering: compatibility review (2026-11-25)

The VitaSpot backend is Rust/librespot, not the official Spotify app or a browser. This review compares representative projects; it is not an exhaustive inventory of every fork.

| Project | Mechanism | Decision |
|---|---|---|
| [uBlock Origin](https://github.com/gorhill/uBlock) | browser content/network filter | adapt narrow request filtering, not browser extension |
| [AdGuard Filters](https://github.com/AdguardTeam/AdguardFilters) | filter lists | adapt exact ad-host matching; no wholesale list import |
| [Pi-hole](https://github.com/pi-hole/pi-hole) | DNS | avoid system-wide DNS changes |
| [Spotify-AdsList](https://github.com/Isaaker/Spotify-AdsList) | domain lists, archived | do not copy entire stale list |
| [spotify-adblock](https://github.com/abba23/spotify-adblock) | Rust CEF interception on Linux | filtering idea only, no CEF on Vita |
| [BlockTheSpot](https://github.com/mrpond/BlockTheSpot) | Windows DLL/CEF patch | incompatible |
| [SpotX-Bash](https://github.com/SpotX-Official/SpotX-Bash) | desktop client patch | incompatible |
| [EeveeSpotifyReborn](https://github.com/whoeevee/EeveeSpotifyReborn) | iOS client tweak | incompatible; project documents Connect limitations |
| [Spicetify](https://github.com/spicetify/cli) | desktop JavaScript customization | incompatible |
| [spotify-no-ads](https://github.com/geranton93/spotify-no-ads) | Spicetify extension | incompatible |
| [Spotify-Scripts](https://github.com/Dabrogost/Spotify-Scripts) | web player scriptlet | incompatible |
| [EZBlocker3](https://github.com/OpenByteDev/EZBlocker3) | desktop ad muting | adapt local muting, not OS-specific code |
| [Blockify](https://github.com/gmdfalk/blockify) | Linux media metadata and muting | adapt explicit classification only |
| [Ad-silence](https://f-droid.org/packages/bluepie.ad_silence/) | Android media events and muting | adapt concept, not Android APIs |
| [librespot](https://github.com/librespot-org/librespot) | Spotify Connect | upstream does not implement free-tier advertising/skip support |

## Implemented

* Rust module `ad_filter.rs`: exact hostname checks for dedicated ad image hosts, explicit ad URI or boolean metadata checks, atomic local audio mute state.
* Image filter before both disk cache and download. Normal image/CDN, metadata, login and audio storage requests are not modified.
* Engine updates ad state from the active Connect snapshot; state is cleared on teardown/reconnection.
* Vita audio output replaces PCM with silence for explicitly identified ads while preserving the playback clock and the physical volume setting. No forced skips, synthetic ad-completion events or account attribute changes.
* Enabled by default. Create an empty file `ux0:data/VitaSpot/disable_ad_filter` and restart to disable both filters. Remove the file and restart to enable again.
* Debug transitions appear in `ux0:data/VitaSpot/vam.log` with the prefix `ad filter:`.
* Unit tests cover positive and negative classifications, including misleading song titles and lookalike domains.

## Limitations and verification

This is a native, best-effort filter, not a Premium unlock. The upstream Connect player may never surface a playable ad URI or ad metadata. The image filter only applies to VitaSpot's own image requests, not all Spotify networking; the PCM filter only applies on Vita. Muted ads still consume time, and embedded podcast advertising may not be identifiable. Neither the unit tests nor the VPK build nor live account playback have been verified in this environment.

Suggested tests: boot and QR pairing; browse and load legitimate cover art; play consecutive songs; confirm hardware volume remains unchanged; compare ad marker transitions to audio; disconnect/reconnect; log out; repeat with the opt-out marker. Test with both Free and Premium accounts and document server-side errors separately. Do not publish as an ad-free release before actual device verification.

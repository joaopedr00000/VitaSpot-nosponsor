#!/usr/bin/env python3
"""Local test of the official Spotify Soloist API; no credentials are handled."""
import argparse
import re
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import urlparse


def track_uri(value):
    if re.fullmatch(r"spotify:track:[A-Za-z0-9]{22}", value):
        return value
    url = urlparse(value)
    if url.scheme == "https" and url.netloc == "open.spotify.com":
        match = re.fullmatch(r"/track/([A-Za-z0-9]{22})/?", url.path)
        if match:
            return "spotify:track:" + match.group(1)
    raise argparse.ArgumentTypeError("Use a Spotify track URI or open.spotify.com/track link.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--soloist", required=True, type=Path,
                        help="Path to the official, separately downloaded executable")
    parser.add_argument("--track", type=track_uri,
                        help="Optional track to test; otherwise inspect existing playback")
    parser.add_argument("--samples", type=int, default=4, choices=range(1, 11))
    args = parser.parse_args()
    exe = args.soloist.expanduser().resolve()
    if not exe.is_file():
        parser.error("Soloist executable not found")
    # Fixed loopback endpoint: never expose its unauthenticated API on the LAN.
    def ctl(*command):
        try:
            result = subprocess.run([str(exe), "ctl", "--ws", "127.0.0.1:9090", *command],
                                    timeout=15, check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            print("Local Soloist command failed:", error, file=sys.stderr)
            return 2
        return result.returncode

    print("Checking the official playback service...", flush=True)
    code = ctl("status")
    if code:
        return code
    if args.track:
        code = ctl("play", args.track)
        if code:
            return code
    for index in range(args.samples):
        print("Playback sample", index + 1, flush=True)
        code = ctl("now")
        if code:
            return code
        if index + 1 < args.samples:
            time.sleep(2)
    print("Commands completed. Confirm audible output on the host yourself.")
    print("A successful command alone does not establish audio playback.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

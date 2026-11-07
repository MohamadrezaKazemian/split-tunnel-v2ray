# Split Tunnel v0.1.0 (experimental)

## What's included

- Desktop split tunneling with an unmodified sing-box TUN sidecar and process rules.
- Import compatible proxy links or subscriptions, select applications, and connect or disconnect.
- Whole-app, executable-path, and search-term rules with live process counts.
- Linux polkit supervision and an experimental Windows implementation.

## Install

Download the matching installer and `SHA256SUMS.txt` from this release, then verify the checksum before installing.

- **Linux x64:** install the `.deb` with `sudo apt install ./<package>.deb`, or make the `.AppImage` executable (`chmod +x <file>.AppImage`) and run it. A polkit agent and `pkexec` are required.
- **Windows x64:** run the NSIS `.exe` installer and accept the UAC prompt when starting the app. WebView2 is required.

### Verify downloads

On Linux, place the downloaded files next to `SHA256SUMS.txt` and run `sha256sum -c SHA256SUMS.txt --ignore-missing`; verify that your installer reports `OK`. On Windows, run `Get-FileHash .\<installer>.exe -Algorithm SHA256` in PowerShell and compare its SHA-256 hash to the matching entry in `SHA256SUMS.txt`.

## Experimental

Linux has had limited manual testing, including an isolated network-namespace cleanup test; full desktop routing and DNS isolation remain unverified. **Windows has not been tested on a real machine.** Installers are unsigned, so Windows SmartScreen or Defender may show a warning. If you have verified the download and decide to run it, SmartScreen may offer **More info > Run anyway**; the exact options depend on system policy.

DNS attribution can be incomplete, UWP/Flatpak/Snap process paths may not match, and helper or child processes outside an app's installation root need separate rules. Process attribution and DNS leaks are not guaranteed. The app runs privileged components and changes system routes. Please report issues with OS and app versions and sanitized logs, removing credentials, subscription links, IP addresses, and personal paths.

## Third-party licensing and disclaimer

The release bundles the unmodified [sing-box 1.14.4](https://github.com/SagerNet/sing-box/releases/tag/v1.14.4) binary, distributed under GPL-3.0-or-later with an additional naming restriction. Obtain [its corresponding upstream source](https://github.com/SagerNet/sing-box/tree/v1.14.4) and see [THIRD_PARTY_NOTICES.md](https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/blob/v0.1.0/THIRD_PARTY_NOTICES.md). Distributors should review the applicable license, source, and notice obligations before publishing binaries.

This app does not provide servers or configurations. Use it lawfully. The author takes no responsibility for misuse.

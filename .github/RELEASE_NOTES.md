# Split Tunnel v0.1.0 (experimental)

## What's included

- Desktop split tunneling with an unmodified sing-box TUN sidecar and process rules.
- Import compatible proxy links or subscriptions, select applications, and connect or disconnect.
- Whole-app, executable-path, and search-term rules with live process counts.
- Whole-system tunnel mode and per-profile TCP, proxy-delay, batch, and live ping.
- Classic and Modern themes, plus redacted log export.
- Release notifications and signed in-app updates for AppImage and NSIS installations.
- Linux polkit supervision and an experimental Windows implementation.

## Downloads

Choose the format for your system. Windows builds are experimental.

| OS and format | Download | Which should I pick? |
| --- | --- | --- |
| Linux x64, Debian package | [split-tunnel-v2ray_0.1.0_linux_amd64.deb](https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/releases/download/v0.1.0/split-tunnel-v2ray_0.1.0_linux_amd64.deb) | Install on Debian or Ubuntu. |
| Linux x64, AppImage | [split-tunnel-v2ray_0.1.0_linux_amd64.AppImage](https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/releases/download/v0.1.0/split-tunnel-v2ray_0.1.0_linux_amd64.AppImage) | Run without installing; make the file executable first. |
| Arch Linux x64 | `sudo pacman -U ./split-tunnel-v2ray_0.1.0_linux_x86_64.pkg.tar.zst` | Install the release package directly. AUR support is coming. |
| Windows x64, installer | [split-tunnel-v2ray_0.1.0_windows_x64-setup.exe](https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/releases/download/v0.1.0/split-tunnel-v2ray_0.1.0_windows_x64-setup.exe) | Use the installer for normal setup. |
| Windows x64, portable | [split-tunnel-v2ray_0.1.0_windows_x64-portable.zip](https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/releases/download/v0.1.0/split-tunnel-v2ray_0.1.0_windows_x64-portable.zip) | Extract and run without installation; administrator access and WebView2 are still required. |

Verify downloads using [SHA256SUMS.txt](https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/releases/download/v0.1.0/SHA256SUMS.txt).

## Install

Download the matching installer and `SHA256SUMS.txt` from this release, then verify the checksum before installing.

- **Linux x64:** install the `.deb` with `sudo apt install ./<package>.deb`, make the `.AppImage` executable (`chmod +x <file>.AppImage`) and run it, or on Arch install the `.pkg.tar.zst` with `sudo pacman -U ./<package>.pkg.tar.zst`. A polkit agent and `pkexec` are required. AUR support is coming; updates are manual until it is live.
- **Windows x64:** run the NSIS `.exe` installer, or extract the portable ZIP and run `split-tunnel-backend.exe` alongside `sing-box.exe`. Both request administrator access and require WebView2.

### Verify downloads

On Linux, place the downloaded files next to `SHA256SUMS.txt` and run `sha256sum -c SHA256SUMS.txt --ignore-missing`; verify that your installer reports `OK`. On Windows, run `Get-FileHash .\<installer>.exe -Algorithm SHA256` in PowerShell and compare its SHA-256 hash to the matching entry in `SHA256SUMS.txt`.

## Experimental

Linux whole-system mode has had desktop testing, and an isolated network-namespace test covered cleanup; other routing and DNS configurations remain incompletely verified. **Windows has not been tested on a real machine.** Windows installers lack Authenticode signing, so SmartScreen or Defender may show a warning; updater signatures are separate from Windows code signing. If you have verified the download and decide to run it, SmartScreen may offer **More info > Run anyway**; the exact options depend on system policy.

DNS attribution can be incomplete, UWP/Flatpak/Snap process paths may not match, and helper or child processes outside an app's installation root need separate rules. Process attribution and DNS leaks are not guaranteed. The app runs privileged components and changes system routes. Please report issues with OS and app versions and sanitized logs, removing credentials, subscription links, IP addresses, and personal paths.

## Third-party licensing and disclaimer

The release bundles the unmodified [sing-box 1.14.4](https://github.com/SagerNet/sing-box/releases/tag/v1.14.4) binary, distributed under GPL-3.0-or-later with an additional naming restriction. Obtain [its corresponding upstream source](https://github.com/SagerNet/sing-box/tree/v1.14.4) and see [THIRD_PARTY_NOTICES.md](https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/blob/v0.1.0/THIRD_PARTY_NOTICES.md). Distributors should review the applicable license, source, and notice obligations before publishing binaries.

This app does not provide servers or configurations. Use it lawfully. The author takes no responsibility for misuse.

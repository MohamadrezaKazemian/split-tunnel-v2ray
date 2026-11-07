# Changelog

This project follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and uses [Semantic Versioning](https://semver.org/).

## [0.2.0] - Unreleased

### Added

- TCP, proxy-delay, batch, and live latency measurements for imported profiles.
- Whole-system tunnel mode with dual-stack routing and proxy DNS.
- Redacted log export, copy, and clear controls.
- Release update checks, signed AppImage and NSIS updates, and manual package-manager instructions.
- Classic desktop theme with a Modern theme option and keyboard-visible focus.

### Changed

- Updated the interface with navigation tabs and a persistent connection status bar.
- The app binary is selected explicitly; the developer ping utility is excluded from bundles.

## [0.1.0] - 2026-11-07

### Added

- Experimental desktop split tunneling using a sing-box TUN sidecar and process rules.
- Import of supported proxy links and subscription URLs, application selection, and connection management.
- Linux polkit supervision and an experimental Windows implementation.
- Whole-app, executable-path, and search-term rules for multi-process applications, with live process counts.

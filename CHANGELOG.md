# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

### Changed

### Fixed

## [0.2.10] - 2026-04-21

### Changed
- Plot series colors now use a larger 12-color palette with hash-based preferred colors plus visible-series collision avoidance, so concurrently visible curves are less likely to reuse the same color.

## [0.2.9] - 2026-04-21

### Added
- The send panel can now be hidden and restored without losing the current layout, and the visibility state is persisted in `config.toml`.

### Changed
- The send panel hide/show interaction now uses a lighter collapse affordance inside the panel plus a compact floating handle when the panel is hidden.

## [0.2.8] - 2026-04-21

### Changed
- The send panel now has a tighter maximum width, and its main editor plus HEX prefix/suffix fields adapt more cleanly within the available panel space.

### Fixed
- Release publishing now purges the gh.123778.xyz CDN cache directly in the main release workflow instead of depending on a separately triggered release event.

## [0.2.7] - 2026-04-21

### Fixed
- Windows startup now cleans stale self-update temporary executables left behind by previous `self-replace` operations in the install directory.
- The serial open failure banner height is now visually aligned more closely with the serial control card beside it.
- The GitHub release workflow now extracts the matching changelog section body correctly instead of publishing empty release notes.

## [0.2.6] - 2026-04-21

### Fixed
- Window icon loading now uses a compiled-in asset so the custom app icon remains available after self-update on Windows.

## [0.2.5] - 2026-04-20

### Changed
- The in-app "new version available" indicator now uses a warmer highlight color so it stands out more clearly from the app's primary blue accent.

## [0.2.4] - 2026-04-20

### Changed
- Windows portable release asset is now named `serial-scope-windows-x86_64-portable.exe` to distinguish it clearly from the installer package.

## [0.2.3] - 2026-04-20

### Fixed
- Windows installer packaging now falls back to the default Inno Setup language when the Simplified Chinese language file is unavailable on the build runner.

## [0.2.2] - 2026-04-20

### Fixed
- Release workflow no longer becomes invalid when optional Windows code-signing secrets are not configured.

## [0.2.1] - 2026-04-20

### Added
- Windows installer packaging with desktop shortcut creation for beginner-friendly installation.
- Optional Windows code-signing support in the release workflow via GitHub secrets.

### Changed
- GitHub Release publishing now uploads all assets and applies changelog notes in one final release step.
- Windows binaries now include richer file metadata for release builds.

### Fixed
- Release changelog publication on GitHub now uses a single final release creation step instead of per-matrix uploads.

## [0.2.0] - 2026-04-20

### Added
- In-app update checks, guided download flow, and restart-after-update support based on GitHub Releases.
- Cross-platform release packaging for Windows, Linux, and macOS in GitHub Actions.
- Plot support for single-value CSV lines such as `9.99` and inline renaming for plot series.

### Changed
- Refined serial monitor, plot view, and send panel layout, follow-state controls, rounded cards, and segmented controls.
- Startup window placement now centers against the active monitor work area and clamps to the available desktop area.
- Release process documentation now requires matching changelog entries before tagging.

### Fixed
- Reopening a port after an earlier access-denied failure now recovers correctly once the external lock is released.
- Plot parsing is more tolerant of mixed debug output, prefixes, and numeric values with units.
- Monitor and plot panel shell sizing, follow-mode behavior, and bounds alignment are now more consistent.

## [0.1.1] - 2026-04-18

### Added
- Save dialogs for receive log and plot CSV export.
- Single-value CSV plotting, including values with units like `9.99V`.
- Plot series renaming from the curve panel.
- Shared main panel shell for monitor, plot, and send views.

### Changed
- Reworked receive monitor follow mode and plot history follow mode.
- Refined panel spacing, rounded cards, segmented controls, and workspace surfaces.
- Simplified plot sidebar controls and improved resize handle styling.
- Release packages now ship as portable binary-only archives.

### Fixed
- More tolerant plot parsing with schema locking for mixed debug output.
- Immediate zoom application while viewing plot history.
- Clearer serial open error messages for unavailable and occupied ports.
- Panel bounds and content sizing consistency across monitor, plot, and send views.

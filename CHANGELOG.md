# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Windows installer packaging with desktop shortcut creation.
- Optional Windows code-signing support in the release workflow.

### Changed
- GitHub Release publishing now uploads all artifacts and changelog notes in one final release step.

### Fixed
- Release changelog publication on GitHub now uses a single release creation step instead of per-matrix uploads.

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

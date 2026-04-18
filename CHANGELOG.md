# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

### Changed

### Fixed

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

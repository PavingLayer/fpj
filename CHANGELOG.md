# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Breaking**: Windows overlay backend now requires WinFSP instead of the
  previous copy-based strategy. Install WinFSP from https://winfsp.dev/ or
  via `choco install winfsp`.
- macOS CI now requires macFUSE (removed fallback to library-only tests).

### Added

- `fpj doctor` command for checking system dependencies and running smoke tests.
- WinFSP-based overlay filesystem on Windows with true copy-on-write semantics,
  whiteout support, and layer merging.
- Hidden `fpj overlay-serve` daemon for hosting WinFSP mounts.

## [0.1.0] - 2026-03-02

### Added

- Named overlay layers with internal upper/work directory management.
- Layer stacking via flat chain resolution (child layers reference locked parents).
- Bind mount support interleaved with overlay layers.
- Atomic mount/unmount with automatic rollback on failure.
- Persistent layout definitions surviving reboots (`fpj restore`).
- Cross-platform backends: Linux (`fuse-overlayfs`/`bindfs`), macOS (macFUSE),
  Windows (WinFSP overlay + NTFS junctions).
- SQLite-backed state persistence.
- Dynamic shell completions for Bash, Zsh, Fish, and PowerShell.
- mdBook documentation published to GitHub Pages.

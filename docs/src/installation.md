# Installation

## From source (all platforms)

Requires the [Rust toolchain](https://rustup.rs/) (1.70+).

```bash
cd tools/fpj
cargo install --path .
```

This places the `fpj` binary in `~/.cargo/bin/`.

## Pre-built binaries

Release binaries are built by CI for:

| Platform | Target | Notes |
|---|---|---|
| Linux x86_64 | `x86_64-unknown-linux-musl` | Statically linked, no runtime deps |
| Linux aarch64 | `aarch64-unknown-linux-musl` | Statically linked |
| macOS Apple Silicon | `aarch64-apple-darwin` | Requires macFUSE for overlay support |
| macOS Intel | `x86_64-apple-darwin` | Requires macFUSE for overlay support |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Junction-based backend by default |

Download from the GitHub Releases page and place on your `PATH`.

## Platform dependencies

### Linux

For overlay mounts (the primary use case):

```bash
# Debian/Ubuntu
sudo apt-get install fuse-overlayfs fuse3

# Fedora/RHEL
sudo dnf install fuse-overlayfs fuse3
```

For bind mounts, the process needs `CAP_SYS_ADMIN` (typically via `sudo`), or you can run in a user namespace.

### macOS

Install macFUSE (5.1+ recommended for the FSKit user-space backend):

```bash
brew install --cask macfuse
brew install bindfs
```

On macOS 15.4+, the FSKit backend runs entirely in user space -- no kernel extension or SIP changes needed.

### Windows

The default Windows backend uses NTFS junction points, which require no additional software.

For advanced overlay support, optionally install [WinFSP](https://winfsp.dev/) and build with:

```bash
cargo install --path . --features winfsp-backend
```

## Data directories

fpj stores its database and layer data at:

- Linux: `~/.local/share/fpj/`
- macOS: `~/Library/Application Support/fpj/`
- Windows: `C:\Users\<user>\AppData\Roaming\fpj\`

Within this directory:

- `fpj.db` -- SQLite database with layer and layout definitions
- `layers/<name>/upper/` -- writable upper directory for each layer
- `layers/<name>/work/` -- overlayfs work directory for each layer

Override the database path with `--db /path/to/custom.db` on any command. Layer data directories are always derived from the database's data directory.

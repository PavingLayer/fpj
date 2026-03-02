# Platform Backends

fpj (File Projector) uses a backend abstraction to support multiple operating systems. Each platform has its own implementation of the `MountBackend` trait, selected at compile time via `#[cfg(target_os)]`.

The backend receives fully resolved paths from the engine -- it has no knowledge of layers, chains, or layouts. It simply mounts and unmounts overlays and binds.

## Backend capabilities

| Operation | Linux | macOS | Windows |
|---|---|---|---|
| Overlay mount | `fuse-overlayfs` | `fuse-overlayfs` via macFUSE | WinFSP overlay daemon |
| Overlay unmount | `fusermount -u` | `umount` | Daemon termination |
| Bind mount | `bindfs` subprocess | `bindfs` subprocess | NTFS junction (`mklink /J`) |
| Bind unmount | `fusermount -u` | `umount` subprocess | `rmdir` junction |
| Is mounted | `/proc/mounts` parsing | `mount` command parsing | Symlink metadata + PID check |
| Copy-up trigger | Touch + remove marker file | Touch + remove marker file | WinFSP handles internally |

## Privilege requirements

| Operation | Linux | macOS | Windows |
|---|---|---|---|
| Overlay (FUSE) | Unprivileged | Requires macFUSE | Requires WinFSP |
| Bind mount | Root or `bindfs` | Unprivileged (bindfs) | Unprivileged (junctions) |

## Diagnostics

Run `fpj doctor` on any platform to check that the required dependencies are
installed and functional. The command runs both dependency checks and live
smoke tests for overlay and bind mount operations.

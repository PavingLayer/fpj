# Platform Backends

fpj (File Projector) uses a backend abstraction to support multiple operating systems. Each platform has its own implementation of the `MountBackend` trait, selected at compile time via `#[cfg(target_os)]`.

The backend receives fully resolved paths from the engine -- it has no knowledge of layers, chains, or layouts. It simply mounts and unmounts overlays and binds.

## Backend capabilities

| Operation | Linux | macOS | Windows |
|---|---|---|---|
| Overlay mount | `fuse-overlayfs` or kernel overlayfs | `fuse-overlayfs` via macFUSE | Copy-based (or WinFSP) |
| Overlay unmount | `fusermount -u` or `umount` | `umount` | Directory removal |
| Bind mount | `mount --bind` via nix syscall | `bindfs` subprocess | NTFS junction (`mklink /J`) |
| Bind unmount | `umount` via nix syscall | `umount` subprocess | `rmdir` junction |
| Is mounted | `/proc/mounts` parsing | `mount` command parsing | Symlink metadata check |
| Copy-up trigger | Touch + remove marker file | Touch + remove marker file | Directory creation |

## Privilege requirements

| Operation | Linux | macOS | Windows |
|---|---|---|---|
| Overlay (FUSE) | Unprivileged | Requires macFUSE | N/A |
| Overlay (kernel) | `CAP_SYS_ADMIN` | N/A | N/A |
| Bind mount | `CAP_SYS_ADMIN` | Unprivileged (bindfs) | Unprivileged (junctions) |

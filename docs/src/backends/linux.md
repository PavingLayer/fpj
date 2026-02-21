# Linux Backend

The Linux backend is the most capable, supporting full overlay and bind mount operations.

## Overlay mounts

fpj tries two strategies in order:

### 1. fuse-overlayfs (preferred)

[fuse-overlayfs](https://github.com/containers/fuse-overlayfs) runs in user space and does not require root privileges. It's the default when `fuse-overlayfs` is on `PATH`.

Requirements:
- `fuse-overlayfs` binary installed
- `/dev/fuse` device available
- `fuse3` package installed

```bash
# Debian/Ubuntu
sudo apt-get install fuse-overlayfs fuse3
```

### 2. Kernel overlayfs (fallback)

If `fuse-overlayfs` is not available, fpj falls back to the kernel's native overlayfs via the `mount()` syscall. This requires `CAP_SYS_ADMIN` (typically via `sudo`).

## Bind mounts

Bind mounts use the `nix` crate's `mount()` function with `MS_BIND` flag, equivalent to `mount --bind`. This requires `CAP_SYS_ADMIN`.

Before bind-mounting into an overlay, fpj triggers a **copy-up** by creating and removing a marker file at the target path. This forces overlayfs to materialize the directory in the upper layer, making it a real directory that can receive bind mounts.

## Mount detection

fpj parses `/proc/mounts` to check whether a path is currently a mount point.

## Unmounting

- Overlay: `fusermount -u` for FUSE mounts, falling back to `umount()` syscall
- Bind: `umount()` syscall via nix

If a path is not currently mounted, unmount is a no-op (not an error).

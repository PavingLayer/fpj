# macOS Backend

The macOS backend uses `fuse-overlayfs` via macFUSE for overlays and `bindfs` for bind mounts.

## Requirements

- **macFUSE 5.1+** -- on macOS 15.4+, the FSKit backend runs entirely in user space (no kernel extension, no SIP changes)
- **bindfs** -- for bind mount support

```bash
brew install --cask macfuse
brew install bindfs
```

## Overlay mounts

fpj invokes `fuse-overlayfs` as a subprocess, same as on Linux:

```
fuse-overlayfs -o lowerdir=...,upperdir=...,workdir=... <mount-point>
```

## Bind mounts

macOS has no native `mount --bind` equivalent. fpj uses [bindfs](https://bindfs.org/) as a FUSE-based alternative:

```
bindfs <source> <target>
```

## Unmounting

Both overlay and bind mounts are unmounted via `umount`:

```
umount <path>
```

## Mount detection

fpj parses the output of the `mount` command to check if a path is mounted.

## Limitations

- Both overlay and bind operations depend on macFUSE being installed and functional
- On macOS versions older than 15.4, macFUSE requires kernel extension approval, which may not be possible in all environments (e.g., MDM-managed machines)
- Performance may be lower than native Linux overlayfs since all I/O goes through FUSE

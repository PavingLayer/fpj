# Windows Backend

The Windows backend provides baseline support using NTFS features, with optional advanced backends available behind feature flags.

## Default: junction points + copy

### Bind mounts

Windows bind mounts use **NTFS junction points** (`mklink /J`), which:

- Work without administrator privileges
- Are transparent to applications (unlike symlinks which some programs detect)
- Only work for directories on the same volume

```
mklink /J <target> <source>
```

Junctions are removed via `rmdir` (which removes the junction link without deleting the source directory).

### Overlay mounts

True overlay filesystems are not natively available on Windows. The default backend uses a **copy-based strategy**: lower directories are recursively copied to the mount point, then the upper directory is copied on top. This is functional but does not provide true copy-on-write semantics.

## Future: advanced backends

True overlay semantics on Windows may be possible through:

- **[WinFSP](https://winfsp.dev/)** — user-mode filesystem framework that could present a merged overlay view similar to Linux's overlayfs.
- **[Windows Projected File System (ProjFS)](https://learn.microsoft.com/en-us/windows/win32/projfs/projected-file-system)** — built into Windows 10+, provides a user-mode API for virtualizing directory contents.

These are not yet implemented. Contributions are welcome.

## Mount detection

The Windows backend checks whether a path is a junction/symlink via filesystem metadata. This only applies to the junction-based bind mount approach.

## Limitations

- The copy-based overlay does not provide live merge semantics -- changes to lower directories after mount are not reflected
- Junction points only work within the same NTFS volume
- No unprivileged overlay mount without WinFSP or ProjFS

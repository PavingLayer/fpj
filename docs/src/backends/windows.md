# Windows Backend

The Windows backend uses [WinFSP](https://winfsp.dev/) for true overlay mount
support and NTFS junction points for bind mounts.

## Requirements

- **WinFSP 2.0+** — user-mode filesystem framework ([download](https://winfsp.dev/rel/) or `choco install winfsp`)

## Overlay mounts

fpj implements a custom overlay filesystem using the WinFSP API. It presents a
merged view of one or more read-only lower directories and one read-write upper
directory at the mount point, with full copy-on-write semantics.

The overlay runs as a background daemon process (`fpj overlay-serve`) that is
automatically spawned by `fpj mount` and terminated by `fpj unmount`. The
daemon's PID is stored in the layer's work directory.

### Layer resolution

When a file is accessed through the mount point:

1. The upper directory is checked first.
2. Lower directories are checked in priority order (first listed = highest priority).
3. If a whiteout marker (`.wh.<filename>`) exists in the upper directory, the
   file is treated as deleted even if it exists in a lower directory.

### Copy-on-write

When a file from a lower directory is modified, it is first copied to the upper
directory. Subsequent operations work on the upper copy, leaving the lower layer
untouched.

## Bind mounts

Windows bind mounts use **NTFS junction points** (`mklink /J`), which:

- Work without administrator privileges
- Are transparent to applications (unlike symlinks which some programs detect)
- Only work for directories on the same volume

```
mklink /J <target> <source>
```

Junctions are removed via `rmdir` (which removes the junction link without
deleting the source directory).

## Mount detection

The backend checks for active junctions via symlink metadata and for overlay
mounts via the daemon PID file.

## Diagnostics

Run `fpj doctor` to verify that WinFSP is installed and that overlay and bind
mount operations work correctly on your system.

## Limitations

- Junction points only work within the same NTFS volume
- WinFSP must be installed system-wide (it is a driver framework, not a
  portable binary)

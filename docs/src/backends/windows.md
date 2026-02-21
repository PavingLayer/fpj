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

## Optional: WinFSP backend

[WinFSP](https://winfsp.dev/) provides a user-mode filesystem framework. Build with the feature flag:

```bash
cargo build --features winfsp-backend
```

This enables a virtual filesystem that presents a merged overlay view, similar to Linux's overlayfs. WinFSP must be installed on the target system.

## Optional: ProjFS backend

[Windows Projected File System](https://learn.microsoft.com/en-us/windows/win32/projfs/projected-file-system) is built into Windows 10+ and provides a user-mode API for virtualizing directory contents. Enable with:

```bash
cargo build --features projfs-backend
```

ProjFS must be enabled:

```powershell
Enable-WindowsOptionalFeature -Online -FeatureName Client-ProjFS -NoRestart
```

## Optional: fpjd

[fpjd](https://github.com/ansnapx/fpjd) is a kernel-mode layered filesystem driver for Windows that provides true overlay capabilities. It requires:

- Kernel driver installation
- Driver signing (for production use)
- Communication via IOCTLs

This is the most powerful option but also the most complex to deploy. It is suitable for controlled environments where driver installation is acceptable.

## Mount detection

The Windows backend checks whether a path is a junction/symlink via filesystem metadata. This only applies to the junction-based bind mount approach.

## Limitations

- The copy-based overlay does not provide live merge semantics -- changes to lower directories after mount are not reflected
- Junction points only work within the same NTFS volume
- No unprivileged overlay mount without WinFSP or ProjFS

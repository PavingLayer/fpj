# fpj

**fpj** is an application-agnostic tool for creating and managing layered filesystem views using overlay mounts and bind mounts. It provides:

- **Named layers** -- first-class entities with internal storage management; you provide only a source directory (or a reference to another layer) and a mount point
- **Layouts** -- persistent recipes that assemble layers and bind mounts into a complete filesystem view, with atomic mount/unmount and rollback on failure
- **Flat chain resolution** -- layers can reference other locked layers as their base, and fpj automatically flattens the entire chain into a single efficient overlay mount
- **Cross-platform** -- a single Rust binary with platform-specific backends for Linux, macOS, and Windows
- **Persistence** -- layer and layout definitions survive reboots in a SQLite database; a single `restore` command remounts everything

## When to use fpj

fpj is useful whenever you need to:

- Compose a directory tree from multiple sources without copying files
- Create writable workspaces on top of read-only base images
- Fork an existing layered environment by locking a layer and building a new layer on top
- Overlay configuration directories onto assembled filesystem views
- Manage all of the above through a CLI or programmatically via subprocess calls

## Quick example

```bash
# Create a layer from a directory source
fpj layer create base-image \
    --source /data/base \
    --mount-point /data/workdirs/base

# Lock the layer so it can be used as a base
fpj layer lock base-image

# Create a child layer on top of the locked base
fpj layer create my-workspace \
    --source @base-image \
    --mount-point /data/workdirs/my-ws

# Create a layout with the layer and a bind mount
fpj layout create my-layout
fpj step add-layer my-layout --layer my-workspace
fpj step add-bind my-layout \
    --source /data/config/dev \
    --target /data/workdirs/my-ws/etc/config

# Mount everything atomically
fpj mount my-layout

# Check status
fpj status my-layout

# Unmount (reverse order)
fpj unmount my-layout
```

## Design principles

1. **Layers are first-class.** Each layer has a unique name, and fpj manages its internal storage (upper/work directories) automatically.
2. **All user-provided paths are absolute.** fpj has no knowledge of directory conventions. The caller specifies mount points and source directories.
3. **Steps are ordered.** The mount sequence in a layout is preserved exactly as defined, allowing arbitrary interleaving of layers and bind mounts.
4. **Mounts are transactional.** If any step fails during mount, all previously completed steps are rolled back.
5. **Definitions are persistent, mounts are not.** Layer and layout definitions survive in SQLite. Actual mounts do not survive reboots but can be restored from the database.
6. **Flat chain resolution.** When a layer references another layer as its base, fpj flattens the entire ancestry into a single overlay lowerdir chain for optimal performance.

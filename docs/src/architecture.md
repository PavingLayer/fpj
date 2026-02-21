# Architecture

fpj is the File Projector CLI for managing layered filesystem layouts.

## Project structure

```
tools/fpj/
├── Cargo.toml
├── docs/                    # This documentation (mdBook)
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── model.rs             # Layer, LayerSource, MountStepDef, Layout
│   ├── database.rs          # SQLite persistence (layers + layouts)
│   ├── engine.rs            # LayoutEngine high-level API + chain resolution
│   ├── operations.rs        # MountTransaction with rollback
│   ├── error.rs             # Error types
│   ├── backend/
│   │   ├── mod.rs           # MountBackend trait + factory
│   │   ├── linux.rs         # fuse-overlayfs + nix
│   │   ├── macos.rs         # macFUSE + bindfs
│   │   └── windows.rs       # Junctions / WinFSP / ProjFS
│   └── cli/
│       ├── mod.rs           # Clap subcommand dispatch
│       ├── layer.rs         # layer commands
│       ├── layout.rs        # layout commands
│       ├── step.rs          # step commands
│       └── mount.rs         # (reserved)
└── tests/
    ├── common/mod.rs        # Test helpers + capability detection
    ├── layout_persistence.rs
    ├── step_ordering.rs
    ├── transaction_rollback.rs
    ├── overlay_mount.rs
    ├── bind_mount.rs
    ├── mixed_mount.rs
    ├── restore.rs
    ├── lock_unlock.rs
    └── cli_integration.rs
```

## Layer diagram

```
┌─────────────────────────────────────────────┐
│                    CLI                       │
│  (layer, layout, step, mount, unmount, ...) │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│               LayoutEngine                   │
│  (layer CRUD, layout CRUD, chain resolution, │
│   mount, unmount, restore, status)           │
└───────┬─────────────────┬───────────────────┘
        │                 │
┌───────▼──────┐  ┌───────▼──────────────────┐
│  Database    │  │  MountTransaction         │
│  (rusqlite)  │  │  (resolve layers,         │
│  - layers    │  │   execute steps,           │
│  - layouts   │  │   rollback on failure)     │
│  - steps     │  └───────┬──────────────────┘
└──────────────┘          │
              ┌───────────▼───────────┐
              │    MountBackend       │
              │    (trait)            │
              └───┬───────┬──────┬───┘
                  │       │      │
           ┌──────▼┐  ┌──▼───┐ ┌▼───────┐
           │ Linux │  │macOS │ │Windows  │
           └───────┘  └──────┘ └─────────┘
```

## Data flow

### Layer creation

1. CLI parses `layer create --source <src> --mount-point <path>`
2. Engine computes internal upper/work paths under `<data_dir>/fpj/layers/<name>/`
3. If source is `@layer-name`, engine verifies the base layer is locked
4. Layer definition is stored in the `layers` table

### Flat chain resolution

When mounting a layer, the engine resolves its full lowerdir chain:

```
Layer "grandchild" (source: @child)
  → child.upper_dir
  → resolve("child")
    → Layer "child" (source: @parent)
      → parent.upper_dir
      → resolve("parent")
        → Layer "parent" (source: /data/base)
          → [/data/base]

Result: [child.upper_dir, parent.upper_dir, /data/base]
```

This produces a flat lowerdir chain passed to a single overlayfs mount call. No nested mounts, optimal performance.

Cycle detection is built in: the resolution tracks visited layer names and errors on revisit.

### Mount

1. CLI parses arguments, calls `LayoutEngine::mount(name)`
2. Engine loads the `Layout` from the database
3. Engine creates a `MountTransaction` with the layout, database, engine, and backend
4. Transaction iterates over steps:
   - `MountStepDef::Layer(name)` → load layer, resolve chain, ensure dirs, call `backend.mount_overlay(...)`
   - `MountStepDef::Bind { source, target }` → call `backend.bind_mount(source, target)`
5. On failure, completed steps are undone in reverse

### Persistence

Two SQLite tables store the core data:

- `layers` -- name, source type/value, mount point, role, upper/work dirs
- `layouts` -- name + creation timestamp
- `mount_steps` -- layout name, position, step type, step definition as JSON

The `layers` table stores all layer metadata directly as columns. The `mount_steps` table uses JSON serialization for step definitions, preserving the `position` column for exact execution order.

## Backend trait

The `MountBackend` trait defines six operations that each platform must implement:

```rust
pub trait MountBackend {
    fn mount_overlay(&self, lower_dirs, upper_dir, work_dir, mount_point) -> Result<()>;
    fn unmount_overlay(&self, mount_point) -> Result<()>;
    fn bind_mount(&self, source, target) -> Result<()>;
    fn unbind_mount(&self, target) -> Result<()>;
    fn is_mounted(&self, path) -> Result<bool>;
    fn ensure_writable_in_overlay(&self, path) -> Result<()>;
}
```

Backend selection happens at compile time via `#[cfg(target_os)]` in `backend/mod.rs::create_backend()`. Each release binary contains exactly one backend.

The backend receives fully resolved absolute paths -- it has no knowledge of layers, layouts, or chains.

## Testing strategy

All tests are integration tests against real filesystems and databases. Tests that need OS-level mount operations check for capabilities at runtime (is `fuse-overlayfs` installed? do we have `CAP_SYS_ADMIN`?) and skip gracefully if not.

Key test categories:
- **Layer CRUD**: creation, removal, lock/unlock, chain resolution, cycle detection
- **Layout persistence**: save/load round-trips, step ordering, cascading deletes
- **Transaction rollback**: failure injection at various step positions
- **Real mounts**: overlay mount/unmount, bind mount/unmount, mixed layouts, chained layers
- **CLI end-to-end**: all subcommands via `assert_cmd`

See `.github/workflows/fpj.yml` for the CI matrix:
- Linux: full coverage (all tests, with sudo + FUSE)
- macOS: conditional (depends on macFUSE installation success)
- Windows: partial (junction-based tests + database tests)

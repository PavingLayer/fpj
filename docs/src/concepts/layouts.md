# Layouts

A **layout** is a named, ordered sequence of mount steps that assemble a complete filesystem view.

## Creating a layout

```bash
fpj layout create my-layout
```

This creates an empty layout with no steps. Steps are added separately via the `step` command.

## Lifecycle

1. **Create** -- `layout create` registers the name in the database
2. **Define steps** -- `step add-layer` and `step add-bind` append operations
3. **Mount** -- `mount` executes all steps atomically
4. **Unmount** -- `unmount` reverses all steps
5. **Restore** -- after a reboot, `restore` re-mounts from the persisted definition
6. **Remove** -- `layout remove` deletes the definition and all its steps

## Listing and inspecting

```bash
# List all layouts
fpj layout list

# Show a layout's steps
fpj layout show my-layout
```

## Persistence

Layout definitions are stored in SQLite. The actual mounts are ephemeral (they don't survive reboots), but the definitions do. Use `fpj restore` to re-mount after a reboot.

```bash
# Restore a specific layout
fpj restore my-layout

# Restore all layouts
fpj restore
```

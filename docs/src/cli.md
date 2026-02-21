# CLI Reference

fpj provides a single binary with subcommands organized into groups.

## Global options

```
fpj [OPTIONS] <COMMAND>

Options:
    --db <PATH>    Path to database file
                   Default: ~/.local/share/fpj/fpj.db
    -h, --help     Print help
```

## Command groups

| Command | Description |
|---|---|
| `layer` | Create, remove, list, inspect, lock, and unlock layers |
| `layout` | Create, remove, list, and inspect layouts |
| `step` | Add and remove mount steps within a layout |
| `mount` | Mount all steps of a layout atomically |
| `unmount` | Unmount all steps in reverse order |
| `restore` | Re-mount layouts from persisted definitions |
| `status` | Show current mount state |

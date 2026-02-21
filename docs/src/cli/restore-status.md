# restore / status

## restore

Re-mount File Projector layouts from their persisted definitions. Use after a reboot to bring layouts back up.

```bash
# Restore a specific layout
fpj restore <LAYOUT>

# Restore all layouts
fpj restore
```

## status

Show the current mount state of a layout's steps.

```bash
fpj status <LAYOUT>
```

Example output:

```
Layout: my-layout
  [●] 0: layer @my-workspace
  [●] 1: bind /data/config/dev -> /data/workdirs/my-ws/etc/config
  [○] 2: bind /data/checkout/ext -> /data/workdirs/my-ws/extensions
```

`●` = mounted, `○` = not mounted.

### JSON output

For programmatic use:

```bash
fpj status <LAYOUT> --json
```

```json
{
  "name": "my-layout",
  "steps": [
    {
      "position": 0,
      "description": "layer @my-workspace",
      "mounted": true
    },
    {
      "position": 1,
      "description": "bind /data/config/dev -> /data/workdirs/my-ws/etc/config",
      "mounted": true
    },
    {
      "position": 2,
      "description": "bind /data/checkout/ext -> /data/workdirs/my-ws/extensions",
      "mounted": false
    }
  ]
}
```

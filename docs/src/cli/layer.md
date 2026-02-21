# layer

Manage layer definitions. Layers are the fundamental building block in fpj -- each one defines an overlay mount with internally managed upper and work directories.

## create

Create a new layer.

```bash
fpj layer create <NAME> --source <SOURCE> --mount-point <PATH>
```

| Argument | Description |
|---|---|
| `<NAME>` | Unique name for the layer |
| `--source` | Either an absolute directory path, or `@layer-name` to reference another layer as base |
| `--mount-point` | Absolute path where the overlay will be mounted |

When `--source` is a `@layer-name` reference, the referenced layer must be locked. fpj verifies this at creation time.

The layer's upper and work directories are managed automatically under `<data_dir>/fpj/layers/<name>/`.

Examples:

```bash
# Layer from a directory
fpj layer create base-image --source /data/base --mount-point /workdirs/base

# Layer referencing another layer (must be locked)
fpj layer create child --source @base-image --mount-point /workdirs/child
```

## remove

Remove a layer definition and its internal directories.

```bash
fpj layer remove <NAME>
```

## list

List all defined layers with their descriptions.

```bash
fpj layer list
```

## show

Show detailed layer information.

```bash
fpj layer show <NAME>
```

Example output:

```
Layer: my-workspace
  Source:      @base-image
  Mount point: /data/workdirs/my-ws
  Role:        writable
  Upper dir:   /home/user/.local/share/fpj/layers/my-workspace/upper
  Work dir:    /home/user/.local/share/fpj/layers/my-workspace/work
```

## lock

Transition a layer from `writable` to `locked`.

```bash
fpj layer lock <NAME>
```

Fails if the layer is not currently `writable`.

## unlock

Transition a layer from `locked` to `writable`.

```bash
fpj layer unlock <NAME>
```

Fails if the layer is not currently `locked`.

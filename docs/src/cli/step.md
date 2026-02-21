# step

Manage mount steps within a File Projector layout. Steps are appended in order and executed sequentially during mount.

## add-layer

Append a layer mount step. The layer must already exist.

```bash
fpj step add-layer <LAYOUT> --layer <LAYER-NAME>
```

| Argument | Description |
|---|---|
| `<LAYOUT>` | Name of the layout to add the step to |
| `--layer` | Name of an existing layer |

## add-bind

Append a bind mount step.

```bash
fpj step add-bind <LAYOUT> --source <PATH> --target <PATH>
```

| Argument | Description |
|---|---|
| `--source` | Absolute path to the external directory |
| `--target` | Absolute path to the mount target |

## remove

Remove a step by its position (0-based index).

```bash
fpj step remove <LAYOUT> --position <N>
```

Subsequent steps shift down to fill the gap.

## list

List all steps in a layout.

```bash
fpj step list <LAYOUT>
```

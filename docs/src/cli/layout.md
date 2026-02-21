# layout

Manage layout definitions for the File Projector. A layout is an ordered sequence of layer mounts and bind mounts.

## create

Create a new empty layout.

```bash
fpj layout create <NAME>
```

## remove

Remove a layout and all its step definitions.

```bash
fpj layout remove <NAME>
```

## list

List all defined layouts.

```bash
fpj layout list
```

## show

Show a layout's steps.

```bash
fpj layout show <NAME>
```

Example output:

```
Layout: my-layout
Steps (3):
  [0] layer @my-workspace
  [1] bind /data/config/dev -> /data/workdirs/my-ws/etc/config
  [2] bind /data/checkout/ext -> /data/workdirs/my-ws/extensions
```

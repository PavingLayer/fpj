# Mount Steps

A layout's mount steps define the exact sequence of filesystem operations. Steps are executed in insertion order during mount and in reverse during unmount.

## Step types

### Layer

A layer step mounts a named layer. The layer must already exist (created via `fpj layer create`).

```bash
fpj step add-layer my-layout --layer my-workspace
```

When executed, fpj:
1. Loads the layer definition from the database
2. Resolves the full lowerdir chain (flattening any base-layer references)
3. Ensures the internal upper/work directories exist
4. Calls the backend to mount the overlay

### Bind

A bind step maps an external directory onto an absolute target path.

```bash
fpj step add-bind my-layout \
    --source /data/config/dev \
    --target /data/workdirs/ws/etc/config
```

The target path typically lies inside an already-mounted layer. fpj triggers a copy-up if the target is inside an overlay so that the bind mount has a real directory to attach to.

## Interleaving

Steps can be freely interleaved. This allows binding directories into a layer's mount point after the layer is mounted:

```bash
fpj step add-layer ws --layer workspace-layer    # step 0
fpj step add-bind ws --source ... --target ...    # step 1: bind into layer
fpj step add-layer ws --layer another-layer       # step 2: another layer
fpj step add-bind ws --source ... --target ...    # step 3: bind into second layer
```

## Managing steps

```bash
# List all steps
fpj step list my-layout

# Remove a step by position
fpj step remove my-layout --position 2
```

Removing a step shifts all subsequent positions down by one.

## Path requirements

All paths in bind steps must be **absolute**. fpj rejects relative paths:

```bash
# This will fail:
fpj step add-bind ws --source relative/path --target /abs/path
# Error: path is not absolute: relative/path
```

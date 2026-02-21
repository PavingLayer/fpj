# Layer Roles

Each layer has a **role** that controls its lifecycle:

| Role | Description | Can be mounted? | Can be used as base? |
|---|---|---|---|
| `writable` | Active layer capturing writes (default) | Yes | No |
| `locked` | Frozen layer, safe for use as base | Yes (read-only) | Yes |

## Role transitions

```
writable -> locked -> writable
```

### Locking

Locking freezes a layer so it can be referenced by other layers as their base:

```bash
fpj layer lock base-image
```

A locked layer guarantees that no further writes will occur to its upper directory, making it safe for other layers to include it in their lowerdir chain.

### Unlocking

Unlocking returns a locked layer to writable state:

```bash
fpj layer unlock base-image
```

This should only be done if no other layers reference this layer as their base.

## Enforcement

When creating a layer with `--source @another-layer`, fpj verifies that the referenced layer is locked. If it is writable, creation fails:

```bash
fpj layer create child --source @parent --mount-point /mp
# Error: base layer 'parent' is not locked (current role: writable)
```

Additionally, at mount time, fpj re-checks the chain: if any base layer has been unlocked since the child was created, the mount will fail.

## Forking pattern

The lock mechanism enables a "forking" workflow:

1. Create a layer from a base directory and work in it
2. Lock the layer to freeze its state
3. Create a new layer referencing the locked one as its base
4. The new layer inherits everything but writes to its own upper directory

```bash
# Create and work in workspace A
fpj layer create ws-a --source /data/base --mount-point /workdirs/a
# ... do work ...

# Lock A and fork to B
fpj layer lock ws-a
fpj layer create ws-b --source @ws-a --mount-point /workdirs/b
```

B sees all of A's files (via the flat chain: `[ws-a/upper, /data/base]`) but writes go to B's own upper directory.

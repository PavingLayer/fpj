# Atomic Operations

fpj treats mounting a layout as a transaction. If any step fails, all previously completed steps are rolled back.

## Mount transaction

When you run `fpj mount my-layout`:

1. Steps are executed in order: step 0, step 1, step 2, ...
2. For layer steps, fpj resolves the full lowerdir chain and ensures internal directories exist
3. If step N fails:
   - Steps N-1, N-2, ..., 0 are undone in reverse order (best-effort)
   - The error from step N is reported
4. If all steps succeed, the layout is fully mounted

```
Mount sequence:         step 0 -> step 1 -> step 2 (fails!)
Rollback:               undo step 1 -> undo step 0
Result:                 clean state, no partial mounts
```

## Unmount transaction

`fpj unmount my-layout` processes steps in reverse:

```
Unmount sequence:       undo step 2 -> undo step 1 -> undo step 0
```

If an unmount step fails, the error is reported immediately. Bind mounts are undone before their underlying layers since they may be mounted on top of layer paths.

## Why atomicity matters

Without transactional mounts, a failure mid-sequence leaves the filesystem in a partially mounted state. This can cause:

- Stale bind mounts pointing into unmounted layers
- Data written to the wrong layer
- Confusion about which mounts are active

fpj avoids all of this by rolling back on failure.

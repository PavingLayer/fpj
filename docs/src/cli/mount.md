# mount / unmount

## mount

Execute all steps of a File Projector layout atomically.

```bash
fpj mount <LAYOUT>
```

Steps are executed in insertion order (step 0 first). For layer steps, fpj (File Projector) resolves the full lowerdir chain and creates internal directories automatically. If any step fails, all completed steps are rolled back in reverse order.

## unmount

Undo all steps of a layout in reverse order.

```bash
fpj unmount <LAYOUT>
```

Bind mounts are undone before layers since they may be mounted on top of layer paths.

# Core Concepts

fpj has three core abstractions:

1. **Layers** -- named, persistent overlay definitions with internal storage management
2. **Layouts** -- ordered sequences of layer mounts and bind mounts, executed atomically
3. **Layer roles** -- the lifecycle state of a layer (writable or locked)

These compose into a simple model: layers define overlay mounts, layouts orchestrate them with bind mounts, and layer references enable stacking.

```
Layer "base-image"
    source: /data/base
    mount:  /data/workdirs/base

Layer "my-workspace"
    source: @base-image (locked)
    mount:  /data/workdirs/ws

Layout "dev"
├── [0] layer @my-workspace
├── [1] bind /config/dev -> /data/workdirs/ws/etc/config
└── [2] bind /checkout/ext -> /data/workdirs/ws/extensions
```

On `fpj mount`, steps execute top-to-bottom. On `fpj unmount`, they execute bottom-to-top (reverse order). If any step fails during mount, all completed steps are rolled back.

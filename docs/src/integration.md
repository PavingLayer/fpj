# Integration Guide

fpj is designed to be used as a standalone CLI tool or integrated into other applications via subprocess calls.

## Subprocess integration (recommended)

The simplest and most portable integration method is invoking the `fpj` binary as a subprocess. This works from any language.

### Python example

```python
import json
import subprocess
from pathlib import Path


def fpj(*args: str, db: Path | None = None) -> subprocess.CompletedProcess:
    cmd = ["fpj"]
    if db:
        cmd.extend(["--db", str(db)])
    cmd.extend(args)
    return subprocess.run(cmd, capture_output=True, text=True, check=True)


# Create a layer
fpj("layer", "create", "base-img",
        "--source", "/data/base",
        "--mount-point", "/data/workdirs/ws")

# Create a layout with the layer and a bind mount
fpj("layout", "create", "my-ws")
fpj("step", "add-layer", "my-ws", "--layer", "base-img")
fpj("step", "add-bind", "my-ws",
        "--source", "/data/config/dev",
        "--target", "/data/workdirs/ws/etc/config")

# Mount
fpj("mount", "my-ws")

# Query status
result = subprocess.run(
    ["fpj", "status", "my-ws", "--json"],
    capture_output=True, text=True
)
status = json.loads(result.stdout)
is_mounted = any(s["mounted"] for s in status["steps"])
```

### Shell script example

```bash
#!/bin/bash
set -euo pipefail

# Create layer and layout
fpj layer create base-img \
    --source /data/base \
    --mount-point /data/workdirs/ws

fpj layout create my-ws
fpj step add-layer my-ws --layer base-img

fpj mount my-ws

# ... do work ...

fpj unmount my-ws
```

## JSON output

The `status --json` command produces machine-readable output for programmatic consumption:

```bash
fpj status my-ws --json
```

```json
{
  "name": "my-ws",
  "steps": [
    {"position": 0, "description": "layer @base-img", "mounted": true},
    {"position": 1, "description": "bind /config -> /data/workdirs/ws/etc/config", "mounted": true}
  ]
}
```

## Database sharing

Multiple tools can share the same fpj database by passing `--db <path>`. This allows different applications to manage their own layers and layouts within a shared namespace.

Be aware that layer and layout names must be unique within a database. Use a naming convention (e.g., prefix with your tool name) to avoid collisions.

## Error handling

fpj (File Projector) uses exit codes to signal success or failure:

| Exit code | Meaning |
|---|---|
| 0 | Success |
| 1 | Error (details on stderr) |

Always check the exit code and capture stderr for error messages.

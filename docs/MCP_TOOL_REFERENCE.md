# MCP Tool Reference — Bevy MCP Server

This document describes the 6 MCP tools exposed by the Bevy MCP Server.

---

## 1. `bevy_ping`

**Description**: Check connectivity to the Bevy BRP server.

**Input**: None (empty params)

**Output**:
```json
{
  "alive": true,
  "methods": { "openrpc": "1.0.0", "info": { "title": "Bevy BRP" }, "methods": ["world.spawn_entity", "world.query", "..."] }
}
```

**Example Usage**:
```
Use bevy_ping to check if the Bevy game is running
```

**Notes**:
- Use this first to verify the MCP server can reach the Bevy game
- Returns the OpenRPC discovery document describing available BRP methods

---

## 2. `bevy_query`

**Description**: Query entities by component types. Returns all entities that have ALL specified components.

**Input**:
```json
{
  "components": ["bevy_ai_remote::AxiomPrimitive", "bevy_transform::components::transform::Transform"]
}
```

**Output**:
```json
{
  "entities": [
    {
      "entity": 4294967298,
      "components": {
        "bevy_ai_remote::AxiomPrimitive": { "primitive_type": "cube" },
        "bevy_transform::components::transform::Transform": {
          "translation": [1.0, 0.0, 1.0],
          "rotation": [0.0, 0.0, 0.0, 1.0],
          "scale": [1.0, 1.0, 1.0]
        }
      }
    }
  ]
}
```

**Example Usage**:
```
Query all primitive shapes: use bevy_query with components ["bevy_ai_remote::AxiomPrimitive"]
```

**Common Component Types**:
- `"bevy_ai_remote::AxiomPrimitive"` — Procedural primitives spawned by the editor
- `"bevy_ai_remote::AxiomRemoteAsset"` — Uploaded GLB/texture assets
- `"bevy_transform::components::transform::Transform"` — Position, rotation, scale

**Notes**:
- This is a **typed wrapper** around `world.query` — params are automatically wrapped in the `data` key
- Opposite behavior from `bevy_rpc_raw` (which is pure pass-through)

---

## 3. `bevy_spawn_primitive`

**Description**: Spawn a primitive 3D object in the Bevy scene.

**Input**:
```json
{
  "primitive_type": "cube",
  "position": [2.0, 1.0, 0.0],
  "rotation": [0.0, 0.0, 0.0, 1.0],
  "scale": [1.0, 1.0, 1.0]
}
```

**Output**:
```json
{
  "entity_id": "4294967299"
}
```

**Example Usage**:
```
Spawn a cube at position (2, 1, 0): use bevy_spawn_primitive with primitive_type "cube" and position [2.0, 1.0, 0.0]
```

**Notes**:
- `primitive_type`: Currently supports `"cube"` (other primitives may be added in future)
- `rotation`: Quaternion `[x, y, z, w]` (default: `[0, 0, 0, 1]` = no rotation)
- `scale`: `[x, y, z]` (default: `[1, 1, 1]`)
- Spawns an entity with `AxiomPrimitive` and `Transform` components

---

## 4. `bevy_upload_asset`

**Description**: Upload a local asset file (GLB, texture) to the Bevy runtime and spawn it in the scene.

**Input**:
```json
{
  "filename": "my_model.glb",
  "data_base64": "Z2xURgIAAAA...",
  "subdir": "Models",
  "translation": [0.0, 0.5, 0.0],
  "rotation": [0.0, 0.0, 0.0, 1.0]
}
```

**Output**:
```json
{
  "entity_id": "4294967300"
}
```

**Example Usage**:
```
Upload my_model.glb to the scene: encode the file as base64, then use bevy_upload_asset with the encoded data and position
```

**Notes**:
- `data_base64`: File bytes encoded as base64 (use standard base64 encoding)
- `subdir`: Optional subdirectory in the game's asset cache (e.g., `"Textures"`, `"Models"`)
- `translation`: Position `[x, y, z]` where the asset will be spawned
- `rotation`: Quaternion `[x, y, z, w]` (default: identity rotation)
- Creates an entity with `AxiomRemoteAsset` component containing the base64 data

---

## 5. `bevy_clear_scene`

**Description**: Clear entities from the Bevy scene by target type.

**Input**:
```json
{
  "target": "all"
}
```

**Targets**:
- `"all"`: Remove all `AxiomPrimitive` AND `AxiomRemoteAsset` entities
- `"assets"`: Remove only `AxiomRemoteAsset` entities (uploaded GLBs/textures)
- `"primitives"`: Remove only `AxiomPrimitive` entities (spawned cubes/etc)

**Output**:
```json
{
  "entities_removed": 5
}
```

**Example Usage**:
```
Clear all primitives from the scene: use bevy_clear_scene with target "primitives"
```

**Notes**:
- Uses `world.query` with `has` filter to find entities, then `world.despawn_entity` to remove matching ones
- Does NOT remove game-native entities (only editor-spawned content)
- Filtering is based on component type matching

---

## 6. `bevy_rpc_raw` ⚠️

**Description**: Send a raw JSON-RPC request directly to the Bevy Remote Protocol endpoint. **Advanced users only.**

**Input**:
```json
{
  "method": "world.spawn_entity",
  "params": {
    "components": {
      "bevy_transform::components::transform::Transform": {
        "translation": [0.0, 0.0, 0.0],
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0]
      }
    }
  }
}
```

**Output**: Raw JSON-RPC response from BRP (format varies by method)

**Example Usage**:
```
ADVANCED: Directly call world.spawn_entity with custom components
```

**⚠️ IMPORTANT DIFFERENCES from `bevy_query`**:
- **Pure pass-through**: Params sent EXACTLY as provided (NO transformation)
- **No `data` key wrapping**: Unlike `bevy_query`, params are NOT wrapped
- **No validation**: You must match Bevy 0.18 BRP schema exactly
- **All BRP methods available**: Includes potentially destructive operations

**When to use**:
- Debugging BRP protocol issues
- Prototyping new operations not yet wrapped in typed tools
- Accessing BRP methods not exposed by other tools

**Prefer typed tools** (`bevy_spawn_primitive`, `bevy_upload_asset`, `bevy_query`) for normal usage.

**Disabled by default** in example configs for safety (see [MCP_SETUP_CODEX_OPENCODE.md](./MCP_SETUP_CODEX_OPENCODE.md) for how to enable).

---

## Component Type Reference

These are the fully-qualified component type strings recognized by Bevy BRP:

| Component | Description | Used By |
|-----------|-------------|---------|
| `bevy_ai_remote::AxiomPrimitive` | Procedural primitive metadata | bevy_spawn_primitive |
| `bevy_ai_remote::AxiomRemoteAsset` | Uploaded asset metadata (base64) | bevy_upload_asset |
| `bevy_transform::components::transform::Transform` | Position, rotation, scale | All spawn operations |

**Example Transform Component**:
```json
{
  "bevy_transform::components::transform::Transform": {
    "translation": [x, y, z],
    "rotation": [x, y, z, w],
    "scale": [x, y, z]
  }
}
```

---

## Workflow Example

1. **Verify connectivity**:
   ```
   Use bevy_ping to check the game is running
   ```

2. **Inspect current scene**:
   ```
   Use bevy_query with components ["bevy_ai_remote::AxiomPrimitive"] to see existing primitives
   ```

3. **Spawn a test primitive**:
   ```
   Use bevy_spawn_primitive to create a cube at position [0, 1, 0]
   ```

4. **Upload a custom model**:
   ```
   Use bevy_upload_asset with my_model.glb (base64 encoded) at position [2, 0, 0]
   ```

5. **Clear when done**:
   ```
   Use bevy_clear_scene with target "all" to remove all editor-spawned content
   ```

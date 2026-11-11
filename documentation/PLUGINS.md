# Writing an Aeolus plugin (API v1)

Aeolus plugins are **declarative**. A plugin is a folder containing an `aeolus-plugin.json` manifest and, optionally, a `README.md`. Aeolus never runs code from a plugin: plugins contribute data and use a small set of host actions, and each capability requires a permission the user approves.

## Manifest

```json
{
  "id": "com.example.my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "What it does.",
  "author": "You",
  "aeolusApi": 1,
  "permissions": ["log", "notifications", "fastflags.presets", "launch.hooks"],
  "dependencies": [{ "id": "com.example.other", "minVersion": "1.2.0" }],
  "contributes": {
    "configuration": [
      { "key": "greeting", "label": "Greeting", "type": "string", "default": "Hi!", "description": "Shown on launch." }
    ],
    "fastFlagPresets": [
      { "name": "Crisp textures", "description": "...", "flags": { "DFIntTextureQualityOverride": 3 } }
    ],
    "hooks": {
      "onEnable":     [{ "action": "log", "message": "enabled" }],
      "onDisable":    [{ "action": "log", "message": "disabled" }],
      "beforeLaunch": [{ "action": "log", "message": "launching" }],
      "afterLaunch":  [{ "action": "notify", "title": "Roblox started", "message": "{{config.greeting}}" }]
    }
  }
}
```

### Rules

- `id`: 3–80 characters of lowercase letters, digits, `.` and `-`. It must match the folder name.
- `aeolusApi`: the plugin API version the plugin targets. It must not be higher than the API version Aeolus supports, which is currently `1`.
- `version`: `major.minor.patch`.
- Configuration field types are `boolean`, `string` and `integer`, and each default must match its type.
- Fast Flag presets are validated like the editor validates flags. The user adds a preset to a profile; plugins never change Fast Flags themselves.

### Permissions

| Permission | Allows |
|---|---|
| `log` | `log` hook actions |
| `notifications` | `notify` hook actions (shown only if the user allows plugin notifications) |
| `fastflags.presets` | offering `fastFlagPresets` on the Fast Flags page |
| `launch.hooks` | running `beforeLaunch` / `afterLaunch` hooks |

A manifest that uses an action without the matching permission is rejected when it is installed.

### Lifecycle

1. **Install**: **Plugins → Install from folder** copies the manifest (and README) into `%APPDATA%\dev.aeolus.app\plugins\<id>`. Nothing else is copied.
2. **Approve**: the user reviews the permissions. Aeolus records the approved permissions and the manifest's SHA-256.
3. **Enable**: `onEnable` hooks run. Dependencies must be installed, meet `minVersion`, and be enabled.
4. **Integrity**: if the manifest changes, its hash no longer matches, so the plugin stops being active and shows **Changed — re-approve**.
5. **Disable / uninstall**: `onDisable` hooks run, and uninstalling removes the folder and the plugin's stored settings.

Errors from hook actions are recorded on the plugin and in the log. They never stop Roblox from launching.

## Example

`plugins/examples/graphics-presets` is a complete plugin. Aeolus places a copy in the plugins folder on first run, disabled until the user approves it.

## Marketplace

There is no marketplace yet. The manifest already carries what one would need: id, version, API compatibility, dependencies and permissions.

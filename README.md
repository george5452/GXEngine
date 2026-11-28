# GXEngine

**Create something extraordinary.**

A lightweight, native 3D game engine built with Rust, designed to deliver advanced graphics, flexible gameplay scripting and efficient resource usage without the overhead of a massive engine.

![Language](https://img.shields.io/badge/Language-Rust-orange)
![Graphics](https://img.shields.io/badge/Graphics-WGPU-blue)
![Physics](https://img.shields.io/badge/Physics-Rapier-purple)
![Scripting](https://img.shields.io/badge/Scripting-GXScript-blueviolet)
![Platform](https://img.shields.io/badge/Platform-Windows-lightgrey)
![Status](https://img.shields.io/badge/Status-Active_Development-green)

Developed by **GXGroup**.

---

## About GXEngine

GXEngine is a native game development environment focused on making 3D game creation accessible while maintaining advanced graphical capabilities.

Our philosophy is simple:

- High-quality graphics.
- Lightweight architecture.
- Efficient memory and storage usage.
- Accessible gameplay programming.
- Minimal unnecessary dependencies.
- Integrated game development tools.

GXEngine is being developed as a general-purpose game engine, not just a rendering demonstration.

## Features

### GX Cinematic Renderer

A custom hybrid rendering system combining signed distance fields and conventional mesh rasterization.

- Physically based rendering.
- SDF-based global illumination.
- Real-time reflections.
- Ambient occlusion.
- BVH-accelerated mesh shadows.
- Artificial point lights.
- Adjustable atmospheric fog.
- HDR lighting and tone mapping.
- Metallic and roughness materials.
- Emissive surfaces.
- Customizable project graphics settings.

### GXScript

GXEngine's integrated text-based gameplay scripting system.

GXScript combines familiar programming syntax with direct access to engine functionality.

- Integrated script editor.
- Syntax highlighting.
- Multiple script tabs.
- Object-attached scripts.
- Project-specific script files.
- Immediate execution during playtesting.
- Custom functions, variables, loops and conditions.
- Runtime debugging console.
- Transform and physics manipulation.
- Keyboard and mouse input.
- Programmable lighting and materials.

Development-time scripting is powered by Rhai. Native Rust compilation for exported gameplay is a planned feature.

### Example

Create an object, attach a GXScript file and write:

```js
func start() {
    print("Hello, GXEngine!");
}

func update(dt) {
    var t = time();

    set_rotation(
        0.0,
        (t * 90.0) % 360.0,
        0.0
    );

    set_position(
        0.0,
        2.0 + sin(t) * 0.5,
        0.0
    );
}
```

Press Play to execute the script directly inside the game.

No external code editor or native compilation is required for normal development-time scripting.

### GXPhysics

Powered by the Rapier physics engine.

- Dynamic and static rigid bodies.
- Collision detection.
- Gravity simulation.
- Physics impulses and velocity control.
- Mesh colliders.
- Configurable world gravity.
- First-person and third-person player controllers.
- Script-controlled physics interactions.

### GXEditor

An integrated native game development interface.

- Scene hierarchy.
- Object inspector.
- Transform gizmos.
- Material controls.
- Project asset browser.
- Model importing.
- Graphics settings.
- Integrated scripting workspace.
- Play and Stop controls.
- Scene saving and loading.
- Project launcher.

### GXStarter

Built-in starter content to help developers create environments quickly.

Includes primitive objects, platforms, walls, stairs, physics objects and material presets.

### GXLighting

Programmable artificial lighting integrated into the cinematic renderer.

- Invisible point light entities.
- Adjustable light intensity.
- Adjustable light range.
- Custom light colors.
- Real-time illumination.
- Script-controlled lighting.
- Mesh and primitive shadow support.

## Supported assets

| Asset | Format | Status |
|---|---|---|
| 3D models | GLB / glTF | Supported |
| Scenes | .gxscene | Supported |
| Gameplay scripts | .gxs | Supported |
| Graphics configuration | JSON | Supported |
| PBR texture maps | PNG / JPG | Planned |
| Audio | WAV / OGG / MP3 | Planned |

## Project structure

Every GXEngine project stores its own scenes, assets, scripts and graphics configuration.

```text
MyGame/
├── Assets/
│   ├── Models/
│   ├── Materials/
│   ├── Textures/
│   ├── Audio/
│   └── Scripts/
│       └── player.gxs
├── Scenes/
│   └── Main.gxscene
└── graphics.json
```

Projects are designed to remain independent and portable.

## Getting started

### Requirements

- Windows computer.
- Rust and Cargo.
- A GPU and graphics driver compatible with WGPU.
- Git.

### Clone the repository

```bash
git clone -b rust https://github.com/george5452/GXEngine.git
cd GXEngine
```

### Build and launch

```bash
cargo run --release
```

### Development build

```bash
cargo check
cargo run
```

The first compilation may take longer because Cargo needs to build the engine's dependencies.

## GXScript API

| Function | Description |
|---|---|
| `start()` | Runs when gameplay starts |
| `update(dt)` | Runs every frame |
| `move(x, y, z)` | Moves the attached object |
| `rotate(x, y, z)` | Rotates the attached object |
| `set_position(x, y, z)` | Sets world position |
| `set_rotation(x, y, z)` | Sets object rotation |
| `set_scale(x, y, z)` | Changes visual scale |
| `set_color(r, g, b)` | Changes material color |
| `set_emission(value)` | Changes material emission |
| `set_velocity(x, y, z)` | Sets physics velocity |
| `add_impulse(x, y, z)` | Applies a physics impulse |
| `set_light_energy(value)` | Changes light intensity |
| `position()` | Returns object position |
| `rotation()` | Returns object rotation |
| `player_position()` | Returns player position |
| `distance_to_player()` | Returns distance to player |
| `key_down(key)` | Detects held input |
| `key_pressed(key)` | Detects newly pressed input |
| `time()` | Returns gameplay time |
| `print(value)` | Writes to the debugging console |

The GXScript API is actively expanding as new engine systems are introduced.

## Technology

| System | Technology |
|---|---|
| Core engine | Rust |
| Graphics | WGPU |
| Shaders | WGSL |
| Editor UI | egui / eframe |
| Physics | Rapier 3D |
| Mathematics | glam |
| Serialization | Serde |
| Model importing | glTF |
| Development scripting | Rhai |

## Development roadmap

### Current foundation

- [x] Native Rust engine
- [x] Integrated scene editor
- [x] Cinematic renderer
- [x] Global illumination
- [x] Physics simulation
- [x] Model importing
- [x] Mesh shadows
- [x] Artificial lighting
- [x] Project management
- [x] GXScript editor
- [x] Object-attached scripting
- [x] Integrated script execution

### Upcoming development

- [ ] Advanced GXScript gameplay API
- [ ] Script file management
- [ ] Collision and trigger events
- [ ] Raycasting
- [ ] Runtime object spawning and destruction
- [ ] PBR texture support
- [ ] Shared mesh and asset resources
- [ ] Renderer optimization
- [ ] Animation system
- [ ] Spatial audio
- [ ] Game UI editor
- [ ] Undo and redo
- [ ] Automatic project recovery
- [ ] Native GXScript compilation
- [ ] Standalone game export
- [ ] Mobile platform support

## Performance philosophy

GXEngine aims to balance graphical quality with efficient RAM and storage consumption.

Future exported games are intended to include only the necessary runtime systems and game assets, without bundling the editor or development tools.

Planned optimizations include shared mesh resources, compressed assets, GPU instancing, compilation caching and native gameplay compilation.

## Repository branches

| Branch | Description |
|---|---|
| `rust` | Current native GXEngine development |
| `main` | Original Electron-based GXEngine |

The Rust branch is the primary focus of development.

## Development status

GXEngine is currently in early development.

Features, APIs and project formats may change as development progresses. Some systems are experimental, and production-ready game exporting is not yet available.

Back up important projects before updating the engine.

## Licensing

Third-party dependencies are subject to their respective licenses. Refer to the repository's license information for GXEngine's own usage and distribution terms.

## Developer

Developed by **GXGroup**.

GitHub: https://github.com/george5452/GXEngine

---

**GXEngine — Create something extraordinary.**
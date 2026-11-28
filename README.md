# GXEngine

### Your imagination. Your engine.

![Language](https://img.shields.io/badge/Language-Rust-orange)
![Graphics](https://img.shields.io/badge/Graphics-WGPU-purple)
![Physics](https://img.shields.io/badge/Physics-Rapier3D-blue)
![Platform](https://img.shields.io/badge/Platform-Windows-0078D4)
![Status](https://img.shields.io/badge/Status-Active_Development-green)
![Version](https://img.shields.io/badge/Version-0.2.0-purple)

**GXEngine** is an independent, native Rust-powered game engine developed by **GeorgeBeeTaong**.

Built with a focus on graphical quality, accessible game development, efficient resource usage, and developer freedom.

GXEngine aims to provide a complete game development environment without unnecessary overhead.

---

## The Foundation Update

The latest development milestone introduces major improvements to game interface design, scene management, material rendering and asset storage.

- GXUI visual interface designer
- Scriptable UI elements
- Independent scene creation and management
- Runtime scene transitions
- Textured GLB and glTF model importing
- Metallic and roughness texture sampling
- Shared mesh asset storage
- Reduced geometry duplication

---

## GX Cinematic

GXEngine features a custom hybrid rendering architecture combining signed distance field rendering and mesh rasterization.

### Rendering features

- Physically based shading
- SDF global illumination approximation
- Real-time reflections
- Soft shadows
- Ambient occlusion
- BVH-based mesh lighting
- Artificial point lights
- Atmospheric fog
- Exposure controls
- ACES tone mapping
- Configurable graphics settings

The renderer is designed to deliver cinematic visuals while maintaining a relatively lightweight native architecture.

---

## GXMaterial

GXMaterial provides material processing and texture support for imported 3D models.

| Feature | Status |
|---|---|
| GLB importing | Supported |
| glTF importing | Supported |
| Embedded textures | Supported |
| External textures | Supported |
| Base-color textures | Supported |
| Metallic maps | Implemented |
| Roughness maps | Implemented |
| Multiple UV channels | Supported |
| GPU texture caching | Supported |
| Normal maps | Planned |
| Emissive maps | Planned |
| Texture mipmaps | Planned |

Material textures are stored separately from scene data, allowing multiple objects to reuse imported resources.

Material appearance and lighting accuracy remain under active development.

---

## GXPhysics

Powered by Rapier3D.

- Dynamic rigid bodies
- Static colliders
- Gravity
- Collision detection
- Imported mesh colliders
- Physics impulses
- Velocity control
- First-person player controllers
- Script-controlled transforms

---

## GXUI

A visual game interface development system integrated directly into GXEngine.

Create main menus, HUDs, loading screens, inventories and custom interfaces.

### Supported elements

- Panels
- Labels
- Buttons
- Progress bars
- Checkboxes

### Editor capabilities

- Visual canvas
- Drag and resize
- Nine anchor presets
- Reference resolution scaling
- Custom colors and transparency
- Adjustable font sizes
- Rounded corners
- Layer ordering
- Element duplication and deletion
- GXScript attachment

GXUI supports standalone interface scenes and overlays within 3D gameplay.

---

## Scene Management

Projects can contain multiple independent scenes.

- Create 3D scenes
- Create UI-only scenes
- Open and switch between scenes
- Save scenes independently
- Automatically save before switching scenes
- Load scenes during gameplay
- Restore the original editor scene after playtesting

A dedicated project startup scene setting is planned.

---

## GXScript

GXScript is GXEngine's integrated gameplay scripting language.

Write scripts directly inside the GXScripter editor and attach them to game objects or UI elements.

The current development runtime is powered by Rhai. Native gameplay compilation is a future development goal.

### 3D gameplay example

```js
func start() {
    print("Hello GXEngine!");
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

### UI scripting example

```js
func clicked() {
    load_scene("Level1");
}
```

### Animated progress bar

```js
func update(dt) {
    set_value(
        sin(time()) * 0.5 + 0.5
    );
}
```

### Interactive interface

```js
func clicked() {
    ui_toggle("Inventory");
}
```

### Dynamic lighting

```js
func update(dt) {
    var t = time();

    set_light_color(
        sin(t) * 0.5 + 0.5,
        sin(t + 2.094) * 0.5 + 0.5,
        sin(t + 4.188) * 0.5 + 0.5
    );

    set_light_energy(50.0);
}
```

### GXScripter editor

- Syntax highlighting
- Line numbers
- Multiple script tabs
- Script explorer
- Syntax validation
- Output console
- Integrated playtesting
- Persistent project scripts

UI and 3D scripts currently use separate runtime contexts. Shared gameplay state is planned.

---

## GXAsset

GXAsset introduces shared mesh resources to reduce unnecessary project storage and memory usage.

Instead of storing complete mesh geometry for every duplicated object, GXEngine can store geometry separately and reference the same asset.

### Features

- Shared mesh geometry using Rust reference counting
- Deduplicated mesh asset files
- Separate texture storage
- Lightweight scene references
- Independent object transforms
- Shared geometry during scene backups
- Compatibility with older inline mesh data

Deleting a scene object does not automatically delete its underlying asset, as other objects or scenes may still depend on it.

Binary compression, GPU instancing and unused asset cleanup are planned improvements.

---

## Project Structure

```text
MyGame/
├── Assets/
│   ├── Meshes/
│   │   └── model.gxmesh
│   ├── Textures/
│   │   ├── diffuse.png
│   │   └── material.png
│   └── Scripts/
│       ├── player.gxs
│       └── menu.gxs
├── Scenes/
│   ├── Main.gxscene
│   ├── MainMenu.gxscene
│   └── Level1.gxscene
└── graphics.json
```

---

## Building from Source

Install Rust and Cargo before building GXEngine.

```bash
git clone -b rust https://github.com/george5452/GXEngine.git
cd GXEngine
cargo run --release
```

For development builds:

```bash
cargo check
cargo run
```

To build an optimized executable:

```bash
cargo build --release
```

The executable will be located inside:

```text
target/release/
```

---

## Technology

| Component | Technology |
|---|---|
| Engine Core | Rust |
| Graphics Backend | WGPU |
| Shader Language | WGSL |
| Editor Framework | egui / eframe |
| Physics | Rapier3D |
| Mathematics | glam |
| Scripting Runtime | Rhai |
| Model Importing | glTF |
| Serialization | Serde |
| Shared Assets | Rust Arc |

GXEngine's native version does not require Electron or a Chromium application runtime.

---

## Development Roadmap

### Core Systems

- [x] Native Rust engine
- [x] Integrated scene editor
- [x] Project management
- [x] Independent scenes
- [x] Runtime scene transitions
- [x] Shared mesh asset foundation
- [ ] Project settings
- [ ] Configurable startup scene
- [ ] Undo and redo
- [ ] Unused asset cleanup

### Rendering

- [x] Hybrid cinematic renderer
- [x] Imported mesh rendering
- [x] Point lights
- [x] Base-color textures
- [x] Metallic and roughness texture implementation
- [ ] Normal mapping
- [ ] Improved material accuracy
- [ ] Texture mipmaps
- [ ] GPU mesh instancing
- [ ] Renderer optimization

### Game Development

- [x] GXScript
- [x] Integrated script editor
- [x] GXUI
- [x] Scriptable UI
- [ ] Shared gameplay state
- [ ] Parent-child object hierarchy
- [ ] Parent-child UI hierarchy
- [ ] Audio system
- [ ] Animation system
- [ ] Input mapping
- [ ] Runtime object spawning

### GX2D

- [ ] Dedicated 2D renderer
- [ ] Sprite rendering
- [ ] 2D cameras
- [ ] Rapier2D physics
- [ ] Sprite animation
- [ ] Tilemaps
- [ ] 2D lighting

### Publishing

- [ ] Standalone game exporting
- [ ] Native GXScript compilation
- [ ] Windows installer
- [ ] Additional platform support

---

## Development Philosophy

GXEngine is being developed around four principles:

**Freedom** — Give developers control over their games.

**Accessibility** — Make game creation approachable without limiting advanced users.

**Efficiency** — Avoid unnecessary resource usage and duplicated assets.

**Quality** — Deliver powerful tools and visually compelling experiences.

---

## Development Status

GXEngine is currently an experimental engine under active development.

Features may change, project formats may evolve, and some systems require additional testing and optimization.

Back up important projects before upgrading to newer development builds.

Windows releases may initially be unsigned. Always obtain builds from official distribution sources.

---

## Repository Branches

| Branch | Description |
|---|---|
| `rust` | Current native GXEngine development |
| `main` | Legacy Electron implementation |

---

## Developer

**GeorgeBeeTaong**

Independent game and software developer.

GXEngine is developed with AI-assisted programming, manual integration, architectural development, debugging and testing.

---

**GXEngine — Create something extraordinary.**

Developed by GeorgeBeeTaong.
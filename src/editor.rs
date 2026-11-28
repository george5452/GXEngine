#[path = "launcher.rs"]
mod launcher;

#[path = "browser.rs"]
mod browser;

#[path = "gizmos.rs"]
mod gizmos;

#[path = "physics.rs"]
mod physics;

#[path = "player.rs"]
mod player;

#[path = "starter.rs"]
mod starter;

#[path = "mesh.rs"]
mod mesh;

#[path = "graphics.rs"]
mod graphics;

#[path = "mesh_light.rs"]
mod mesh_light;

#[path = "lighting.rs"]
mod lighting;

#[path = "scripting.rs"]
mod scripting;

#[path = "gui.rs"]
mod gui;

#[path = "scenes.rs"]
mod scenes;

#[path = "materials.rs"]
mod materials;

#[path = "assets.rs"]
mod assets;

use bytemuck::{Pod, Zeroable};
use eframe::{egui, egui_wgpu};
use egui_wgpu::CallbackTrait;
use gizmos::{GizmoDrag, GizmoMode};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wgpu::util::DeviceExt;

const MAX_OBJECTS: usize = 64;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
enum Shape {
    Cube,
    Sphere,
    Cylinder,
    Capsule,
    Mesh,
}

#[derive(Clone, Serialize, Deserialize)]
struct Object {
    name: String,
    shape: Shape,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
    color: [f32; 3],
    metallic: f32,
    roughness: f32,
    emission: f32,
    dynamic: bool,

    #[serde(default)]
    mesh_asset: Option<String>,

    #[serde(default)]
    light_only: bool,

    #[serde(default)]
    light: lighting::Light,

    #[serde(default)]
    meshes: std::sync::Arc<Vec<mesh::MeshData>>,

    #[serde(default)]
    player_mode: u8,

    #[serde(default = "default_walk_speed")]
    walk_speed: f32,

    #[serde(default = "default_sprint_speed")]
    sprint_speed: f32,

    #[serde(default = "default_jump_speed")]
    jump_speed: f32,

    #[serde(default = "default_camera_distance")]
    camera_distance: f32,

    #[serde(default)]
    script: String,
}

fn default_walk_speed() -> f32 {
    5.0
}
fn default_sprint_speed() -> f32 {
    9.0
}
fn default_jump_speed() -> f32 {
    8.0
}
fn default_camera_distance() -> f32 {
    6.0
}

impl Object {
    fn new(shape: Shape) -> Self {
        Self {
            script: String::new(),
            light_only: false,
            light: lighting::Light::default(),
            name: format!("{:?}", shape),
            shape,
            position: [0.0, 1.5, 0.0],
            rotation: [0.0; 3],
            scale: [1.0; 3],
            color: [0.65, 0.42, 0.85],
            metallic: 0.1,
            roughness: 0.35,
            emission: 0.0,
            dynamic: true,
            meshes: std::sync::Arc::new(Vec::new()),
            mesh_asset: None,

            player_mode: 0,
            walk_speed: 5.0,
            sprint_speed: 9.0,
            jump_speed: 8.0,
            camera_distance: 6.0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Environment {
    exposure: f32,
    sun: f32,
    gi: f32,
    gi_distance: f32,
    ao: f32,
    fog: f32,
    shadow_softness: f32,
    reflections: f32,
    glow: f32,
    gravity: f32,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            sun: 3.0,
            gi: 0.7,
            gi_distance: 6.0,
            ao: 1.0,
            fog: 0.012,
            shadow_softness: 12.0,
            reflections: 0.8,
            glow: 0.15,
            gravity: 9.81,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Scene {
    #[serde(default)]
    canvas: gui::Canvas,
    objects: Vec<Object>,
    environment: Environment,
}

impl Default for Scene {
    fn default() -> Self {
        let mut cube = Object::new(Shape::Cube);
        cube.position = [-1.2, 0.5, 0.0];

        let mut sphere = Object::new(Shape::Sphere);
        sphere.position = [1.2, 0.7, 0.0];
        sphere.color = [0.8, 0.25, 0.12];
        sphere.metallic = 0.65;

        Self {
            objects: vec![cube, sphere],
            environment: Environment::default(),
            canvas: gui::Canvas::default(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GPUObject {
    position: [f32; 4],
    scale: [f32; 4],
    color: [f32; 4],
    material: [f32; 4],
    rotation: [f32; 4],
    light_color: [f32; 4],
    light_settings: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    camera: [f32; 4],
    forward: [f32; 4],
    right: [f32; 4],
    up: [f32; 4],
    settings: [f32; 4],
    effects: [f32; 4],
    extra: [f32; 4],
    objects: [GPUObject; MAX_OBJECTS],
}

struct GPUResources {
    lighting: mesh_light::MeshLighting,
    mesh: mesh::MeshGPU,
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

struct ViewportCallback {
    pbr_paths: Vec<PathBuf>,
    texture_paths: Vec<PathBuf>,
    vertices: Vec<mesh::MeshVertex>,
    uniforms: Uniforms,
}

impl CallbackTrait for ViewportCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let gpu: &mut GPUResources = resources.get_mut().unwrap();

        gpu.mesh
            .textures
            .upload(_device, queue, &self.texture_paths);
        gpu.mesh
            .pbr_textures
            .upload(_device, queue, &self.pbr_paths);
        gpu.mesh.upload(_device, &self.vertices);
        gpu.lighting.upload(queue, &self.vertices);

        queue.write_buffer(&gpu.buffer, 0, bytemuck::bytes_of(&self.uniforms));

        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let gpu: &GPUResources = resources.get().unwrap();

        pass.set_pipeline(&gpu.pipeline);
        pass.set_bind_group(0, &gpu.bind_group, &[]);
        pass.draw(0..3, 0..1);
        gpu.mesh.draw(pass, &gpu.bind_group);
    }
}

fn initialize_gpu(cc: &eframe::CreationContext<'_>) {
    let state = cc.wgpu_render_state.as_ref().unwrap();
    let device = &state.device;

    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("GX Scene Buffer"),
        contents: bytemuck::bytes_of(&Uniforms::zeroed()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let lighting = mesh_light::MeshLighting::new(device);
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("GX Scene Layout"),

        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,

                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,

                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },

                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,

                visibility: wgpu::ShaderStages::FRAGMENT,

                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },

                    has_dynamic_offset: false,
                    min_binding_size: None,
                },

                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("GX Scene Bind Group"),

        layout: &layout,

        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: lighting.buffer.as_entire_binding(),
            },
        ],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("GX Cinematic"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n{}",
                include_str!("cinematic.wgsl"),
                include_str!("mesh_light.wgsl"),
            )
            .into(),
        ),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("GX Pipeline Layout"),
        bind_group_layouts: &[&layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("GX Cinematic Renderer"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: state.target_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview: None,
        cache: None,
    });

    let mesh = mesh::MeshGPU::new(device, &layout, state.target_format);

    state
        .renderer
        .write()
        .callback_resources
        .insert(GPUResources {
            lighting,
            mesh,
            pipeline,
            buffer,
            bind_group,
        });

    println!("[GXRenderer] Cinematic initialized");
}

struct GXEngine {
    scene_manager: scenes::SceneManager,
    gui_editor: gui::GUIEditor,
    scripter: scripting::GXScripter,
    scene: Scene,
    backup: Option<Scene>,
    physics_world: Option<physics::GXPhysics>,
    selected: Option<usize>,
    camera: Vec3,
    yaw: f32,
    pitch: f32,
    playing: bool,
    mouse_locked: bool,
    editor_camera: Vec3,
    editor_yaw: f32,
    editor_pitch: f32,
    environment_selected: bool,
    status: String,
    path: String,
    fps: f32,
    launcher_open: bool,
    project_name: String,
    project_dir: PathBuf,
    browser_dir: PathBuf,
    template: usize,
    starter_tab: usize,
    starter_search: String,
    browser_tab: usize,

    gizmo_mode: GizmoMode,
    gizmo_drag: Option<GizmoDrag>,
}

impl GXEngine {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        initialize_gpu(cc);

        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        Self {
            scene_manager: scenes::SceneManager::default(),
            gui_editor: gui::GUIEditor::default(),
            scripter: scripting::GXScripter::default(),
            scene: Scene::default(),
            backup: None,
            physics_world: None,
            selected: Some(0),
            camera: Vec3::new(5.0, 3.0, 7.0),
            yaw: -2.52,
            pitch: -0.28,
            playing: false,
            mouse_locked: false,
            editor_camera: Vec3::ZERO,
            editor_yaw: 0.0,
            editor_pitch: 0.0,
            environment_selected: false,
            status: "GXEngine initialized".into(),
            path: "scene.gxscene".into(),
            fps: 60.0,

            launcher_open: true,
            project_name: "My Game".into(),
            project_dir: PathBuf::from("projects"),
            browser_dir: PathBuf::from("projects"),
            template: 1,
            starter_tab: 0,
            starter_search: String::new(),
            browser_tab: 0,

            gizmo_mode: GizmoMode::Move,
            gizmo_drag: None,
        }
    }

    fn spawn(&mut self, shape: Shape) {
        if self.scene.objects.len() >= MAX_OBJECTS {
            self.status = "Scene object limit reached".into();
            return;
        }

        self.scene.objects.push(Object::new(shape));

        self.selected = Some(self.scene.objects.len() - 1);

        self.environment_selected = false;
    }

    fn play(&mut self) {
        if let Err(error) = self.scripter.save_pending(&self.project_dir) {
            self.status = format!("Cannot start game: {}", error);

            return;
        }
        self.backup = Some(self.scene.clone());
        self.editor_camera = self.camera;
        self.editor_yaw = self.yaw;
        self.editor_pitch = self.pitch;

        let mut world = physics::GXPhysics::new(&self.scene);

        if let Some(player) = self.scene.objects.iter().find(|o| o.player_mode != 0) {
            world.spawn_player(player.position);

            self.yaw = -std::f32::consts::FRAC_PI_2;
            self.pitch = 0.0;
        }

        self.physics_world = Some(world);
        self.playing = true;
        self.status = "Game running".into();
    }

    fn stop(&mut self) {
        self.physics_world = None;
        self.camera = self.editor_camera;
        self.yaw = self.editor_yaw;
        self.pitch = self.editor_pitch;

        if let Some(scene) = self.backup.take() {
            self.scene = scene;
        }

        self.playing = false;
        self.selected = None;
        self.status = "Edit mode".into();
    }

    fn save(&mut self) {
        match assets::save_scene(
            &self.scene,
            std::path::Path::new(&self.path),
            &self.project_dir,
        ) {
            Ok(()) => {
                self.status = "Scene saved".into();
            }

            Err(error) => {
                self.status = error;
            }
        }
    }

    fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    fn uniforms(&self, aspect: f32) -> Uniforms {
        let mut data = Uniforms::zeroed();

        let forward = self.forward();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();

        data.camera = self.camera.extend(aspect).to_array();
        data.forward = forward.extend(0.0).to_array();
        data.right = right.extend(0.0).to_array();
        data.up = up.extend(0.0).to_array();

        let env = &self.scene.environment;

        data.settings = [
            self.scene.objects.len().min(MAX_OBJECTS) as f32,
            env.exposure,
            env.sun,
            env.gi,
        ];

        data.effects = [env.ao, env.fog, env.shadow_softness, env.reflections];

        data.extra = [
            env.gi_distance,
            env.glow,
            self.selected.map(|i| i as f32).unwrap_or(-1.0),
            0.0,
        ];

        for (i, object) in self.scene.objects.iter().take(MAX_OBJECTS).enumerate() {
            if self.playing && object.player_mode == 1 {
                data.objects[i].position = [0.0, -1000000.0, 0.0, 3.0];

                data.objects[i].scale = [0.01, 0.01, 0.01, 0.0];

                continue;
            }
            let shape = if object.light_only {
                5.0
            } else {
                match object.shape {
                    Shape::Cube => 0.0,
                    Shape::Sphere => 1.0,
                    Shape::Cylinder => 2.0,
                    Shape::Capsule => 3.0,
                    Shape::Mesh => 4.0,
                }
            };

            let q = Quat::from_euler(
                glam::EulerRot::XYZ,
                object.rotation[0].to_radians(),
                object.rotation[1].to_radians(),
                object.rotation[2].to_radians(),
            )
            .inverse();

            data.objects[i] = GPUObject {
                position: [
                    object.position[0],
                    object.position[1],
                    object.position[2],
                    shape,
                ],
                scale: [
                    object.scale[0].max(0.05),
                    object.scale[1].max(0.05),
                    object.scale[2].max(0.05),
                    0.0,
                ],
                color: [object.color[0], object.color[1], object.color[2], 1.0],
                material: [object.metallic, object.roughness, object.emission, 0.0],
                rotation: q.to_array(),
                light_color: [
                    object.light.color[0],
                    object.light.color[1],
                    object.light.color[2],
                    object.light.energy,
                ],

                light_settings: [
                    if object.light.enabled { 1.0 } else { 0.0 },
                    object.light.kind as f32,
                    object.light.range,
                    0.0,
                ],
            };
        }

        data
    }

    fn physics(&mut self, dt: f32) {
        if !self.playing {
            return;
        }

        if let Some(world) = &mut self.physics_world {
            world.update(&mut self.scene, dt);
        }
    }

    fn save_project(&mut self) {
        if self.playing || self.launcher_open {
            return;
        }

        if let Err(error) = self.scripter.save_pending(&self.project_dir) {
            self.status = format!("Script save failed: {}", error);

            return;
        }

        self.save();
        self.save_graphics_settings();
    }
}

fn vector_editor(ui: &mut egui::Ui, name: &str, value: &mut [f32; 3]) {
    ui.label(name);

    ui.horizontal(|ui| {
        for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
            ui.add(
                egui::DragValue::new(&mut value[i])
                    .prefix(format!("{} ", axis))
                    .speed(0.05),
            );
        }
    });
}

impl eframe::App for GXEngine {
    fn on_exit(&mut self) {
        if !self.launcher_open {
            if self.playing {
                self.stop();
            }

            self.save_project();
        }
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.launcher_open {
            self.show_launcher(ctx);
            return;
        }
        let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.05);

        self.fps = self.fps * 0.95 + (1.0 / dt) * 0.05;

        self.physics(dt);
        self.player_update(ctx);
        self.scripting_update(ctx, dt);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("GXEngine").color(egui::Color32::from_rgb(185, 150, 255)),
                );

                ui.separator();

                if !self.playing {
                    ui.selectable_value(&mut self.gui_editor.open, false, "3D");

                    ui.selectable_value(&mut self.gui_editor.open, true, "UI");
                }

                if ui.button("Save Project").clicked() && !self.playing {
                    self.save_project();
                }

                if ui.button("Projects").clicked() && !self.playing {
                    self.save_project();
                    self.launcher_open = true;
                }

                if ui.button("Scripter").clicked() && !self.playing {
                    self.scripter.open = true;
                }

                self.scene_toolbar(ui);

                ui.separator();

                if self.playing {
                    if ui.button("■ Stop").clicked() {
                        self.unlock_mouse(ctx);
                        self.stop();
                    }
                } else if ui.button("▶ Play").clicked() {
                    self.play();
                }

                ui.separator();
                ui.menu_button("Graphics", |ui| {
                    self.graphics_settings(ui);
                });
                ui.separator();
                self.gizmo_toolbar(ui);
                ui.separator();
                self.player_toolbar(ui);
            });
        });

        if self.playing && self.scene.canvas.ui_only {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());

                self.gui_runtime(ui, rect);
            });

            self.scripting_window(ctx);
            ctx.request_repaint();
            return;
        }

        if self.gui_editor.open && !self.playing {
            self.gui_workspace(ctx);
            self.scripting_window(ctx);
            ctx.request_repaint();
            return;
        }

        egui::SidePanel::left("hierarchy")
            .default_width(190.0)
            .show(ctx, |ui| {
                ui.heading("Hierarchy");
                ui.separator();

                if ui
                    .selectable_label(self.environment_selected, "Environment")
                    .clicked()
                {
                    self.environment_selected = true;
                    self.selected = None;
                }

                ui.separator();

                ui.add_enabled_ui(!self.playing, |ui| {
                    ui.menu_button("+ Add Object", |ui| {
                        if ui.button("Point Light").clicked() {
                            self.spawn_light();
                            ui.close_menu();
                        }

                        ui.separator();
                        if ui.button("Cube").clicked() {
                            self.spawn(Shape::Cube);
                            ui.close_menu();
                        }

                        if ui.button("Sphere").clicked() {
                            self.spawn(Shape::Sphere);
                            ui.close_menu();
                        }

                        if ui.button("Cylinder").clicked() {
                            self.spawn(Shape::Cylinder);
                            ui.close_menu();
                        }
                    });
                });

                for (i, object) in self.scene.objects.iter().enumerate() {
                    if ui
                        .selectable_label(self.selected == Some(i), &object.name)
                        .clicked()
                    {
                        self.selected = Some(i);
                        self.environment_selected = false;
                    }
                }

                ui.separator();

                if !self.playing {
                    if let Some(index) = self.selected {
                        if ui.button("Duplicate").clicked()
                            && self.scene.objects.len() < MAX_OBJECTS
                            && self.scene.objects[index].player_mode == 0
                        {
                            let mut obj = self.scene.objects[index].clone();

                            obj.name.push_str(" Copy");
                            obj.position[0] += 1.5;

                            self.scene.objects.push(obj);
                        }

                        if ui.button("Delete").clicked() {
                            self.scene.objects.remove(index);
                            self.selected = None;
                        }
                    }
                }
            });

        egui::SidePanel::right("inspector")
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                ui.separator();

                if let Some(index) = self.selected {
                    if index < self.scene.objects.len() {
                        self.script_inspector(ui, index);
                    }
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.environment_selected {
                        let env = &mut self.scene.environment;

                        ui.heading("Cinematic Environment");

                        ui.add(egui::Slider::new(&mut env.exposure, 0.1..=3.0).text("Exposure"));

                        ui.add(egui::Slider::new(&mut env.sun, 0.0..=8.0).text("Sun energy"));

                        ui.separator();
                        ui.heading("SDF Indirect Lighting");

                        ui.add(egui::Slider::new(&mut env.gi, 0.0..=2.0).text("GI strength"));

                        ui.add(
                            egui::Slider::new(&mut env.gi_distance, 1.0..=15.0).text("GI distance"),
                        );

                        ui.separator();
                        ui.heading("Effects");

                        ui.add(egui::Slider::new(&mut env.ao, 0.0..=2.0).text("Ambient occlusion"));

                        ui.add(egui::Slider::new(&mut env.fog, 0.0..=0.1).text("Fog"));

                        ui.add(
                            egui::Slider::new(&mut env.shadow_softness, 2.0..=40.0)
                                .text("Shadow sharpness"),
                        );

                        ui.add(
                            egui::Slider::new(&mut env.reflections, 0.0..=1.0).text("Reflections"),
                        );

                        ui.add(egui::Slider::new(&mut env.glow, 0.0..=1.0).text("Emissive glow"));

                        ui.separator();

                        ui.add(egui::Slider::new(&mut env.gravity, 0.0..=30.0).text("Gravity"));

                        if ui.button("Reset Environment").clicked() {
                            *env = Environment::default();
                        }
                    } else if let Some(index) = self.selected {
                        if let Some(obj) = self.scene.objects.get_mut(index) {
                            ui.text_edit_singleline(&mut obj.name);

                            ui.separator();
                            ui.heading("Transform");

                            vector_editor(ui, "Position", &mut obj.position);

                            vector_editor(ui, "Rotation", &mut obj.rotation);

                            vector_editor(ui, "Scale", &mut obj.scale);

                            ui.separator();
                            ui.heading("Material");

                            ui.color_edit_button_rgb(&mut obj.color);

                            ui.add(
                                egui::Slider::new(&mut obj.metallic, 0.0..=1.0).text("Metallic"),
                            );

                            ui.add(
                                egui::Slider::new(&mut obj.roughness, 0.02..=1.0).text("Roughness"),
                            );

                            ui.add(
                                egui::Slider::new(&mut obj.emission, 0.0..=10.0).text("Emission"),
                            );

                            ui.separator();
                            lighting::light_inspector(ui, obj);

                            ui.separator();
                            ui.heading("Physics");
                            if obj.player_mode == 0 {
                                ui.checkbox(&mut obj.dynamic, "Dynamic");
                            } else {
                                ui.separator();
                                ui.heading("Character Controller");

                                ui.selectable_value(&mut obj.player_mode, 1, "First Person");

                                ui.selectable_value(&mut obj.player_mode, 2, "Third Person");

                                ui.add(
                                    egui::Slider::new(&mut obj.walk_speed, 1.0..=20.0)
                                        .text("Walk Speed"),
                                );

                                ui.add(
                                    egui::Slider::new(&mut obj.sprint_speed, 1.0..=30.0)
                                        .text("Sprint Speed"),
                                );

                                ui.add(
                                    egui::Slider::new(&mut obj.jump_speed, 1.0..=20.0)
                                        .text("Jump Speed"),
                                );

                                if obj.player_mode == 2 {
                                    ui.add(
                                        egui::Slider::new(&mut obj.camera_distance, 2.0..=15.0)
                                            .text("Camera Distance"),
                                    );
                                }
                            }
                        }
                    } else {
                        ui.label("Select an object");
                    }
                });
            });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.separator();
                ui.label(format!("{:.0} FPS", self.fps));
                ui.separator();
                ui.label(format!("{} objects", self.scene.objects.len()));
                ui.separator();
                ui.label("GX Cinematic");
            });
        });
        self.show_browser(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let size = ui.available_size();

            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

            if !(self.playing && self.scene.objects.iter().any(|o| o.player_mode != 0))
                && (response.hovered() || response.dragged())
            {
                let forward = self.forward();
                let right = forward.cross(Vec3::Y).normalize();

                ctx.input(|input| {
                    let speed = if input.modifiers.shift { 12.0 } else { 4.0 } * dt;

                    if input.key_down(egui::Key::W) {
                        self.camera += forward * speed;
                    }

                    if input.key_down(egui::Key::S) {
                        self.camera -= forward * speed;
                    }

                    if input.key_down(egui::Key::A) {
                        self.camera -= right * speed;
                    }

                    if input.key_down(egui::Key::D) {
                        self.camera += right * speed;
                    }

                    if input.key_down(egui::Key::E) {
                        self.camera.y += speed;
                    }

                    if input.key_down(egui::Key::Q) {
                        self.camera.y -= speed;
                    }

                    if input.pointer.secondary_down() {
                        let delta = input.pointer.delta();

                        self.yaw += delta.x * 0.004;

                        self.pitch = (self.pitch - delta.y * 0.004).clamp(-1.5, 1.5);
                    }
                });
            }

            let over_ui = ctx.input(|i| {
                i.pointer
                    .hover_pos()
                    .map(|p| self.gui_hit(rect, p))
                    .unwrap_or(false)
            });

            if !self.playing || self.mouse_locked || !over_ui {
                self.player_mouse(ctx, &response);
            }

            let aspect = rect.width() / rect.height().max(1.0);

            let uniforms = self.uniforms(aspect);

            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                rect,
                ViewportCallback {
                    uniforms,
                    vertices: self.mesh_vertices(),
                    texture_paths: self.mesh_texture_paths(),
                    pbr_paths: self.mesh_pbr_paths(),
                },
            ));

            self.draw_gizmos(ui, rect);

            if self.playing {
                self.gui_runtime(ui, rect);
            }

            if self.playing {
                let message = if self.mouse_locked {
                    "ESC to release mouse"
                } else {
                    "Click viewport to control player"
                };

                ui.painter().text(
                    rect.center_bottom() - egui::vec2(0.0, 25.0),
                    egui::Align2::CENTER_CENTER,
                    message,
                    egui::FontId::proportional(16.0),
                    egui::Color32::WHITE,
                );

                if self.mouse_locked {
                    let center = rect.center();

                    ui.painter()
                        .circle_filled(center, 2.0, egui::Color32::WHITE);
                }
            }
            ui.painter().text(
                rect.left_top() + egui::vec2(12.0, 12.0),
                egui::Align2::LEFT_TOP,
                "Perspective | Cinematic",
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        });

        self.scripting_window(ctx);
        ctx.request_repaint();
    }
}

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        depth_buffer: 32,
        viewport: egui::ViewportBuilder::default()
            .with_title("GXEngine")
            .with_inner_size([1440.0, 850.0]),
        ..Default::default()
    };

    eframe::run_native(
        "GXEngine",
        options,
        Box::new(|cc| Ok(Box::new(GXEngine::new(cc)))),
    )
}

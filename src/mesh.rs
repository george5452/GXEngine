use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct MeshData {
    #[serde(default)]
    pub uvs: Vec<[f32; 2]>,

    #[serde(default)]
    pub texture: Option<String>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,

    #[serde(default)]
    pub pbr_texture: Option<String>,

    #[serde(default)]
    pub pbr_uvs: Vec<[f32; 2]>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct MeshVertex {
    pub(super) position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    material: [f32; 2],
    uv: [f32; 2],
    texture_layer: f32,
    pbr_uv: [f32; 2],
    pbr_layer: f32,
}

pub(super) struct MeshGPU {
    pub pbr_textures: materials::MaterialTextures,
    pub textures: materials::MaterialTextures,
    pipeline: wgpu::RenderPipeline,
    buffer: Option<wgpu::Buffer>,
    count: u32,
}

fn import_node(
    node: gltf::Node<'_>,
    parent: glam::Mat4,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
    project: &std::path::Path,
    meshes: &mut Vec<MeshData>,
) {
    let local = glam::Mat4::from_cols_array_2d(&node.transform().matrix());

    let world = parent * local;

    let normal_matrix = glam::Mat3::from_mat4(world).inverse().transpose();

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                continue;
            }

            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));

            let Some(positions) = reader.read_positions() else {
                continue;
            };

            let positions: Vec<[f32; 3]> = positions
                .map(|p| world.transform_point3(Vec3::from_array(p)).to_array())
                .collect();

            let mut indices: Vec<u32> = if let Some(indices) = reader.read_indices() {
                indices.into_u32().collect()
            } else {
                (0..positions.len() as u32).collect()
            };

            if world.determinant() < 0.0 {
                for triangle in indices.chunks_exact_mut(3) {
                    triangle.swap(1, 2);
                }
            }

            let normals: Vec<[f32; 3]> = if let Some(normals) = reader.read_normals() {
                normals
                    .map(|n| {
                        (normal_matrix * Vec3::from_array(n))
                            .normalize_or_zero()
                            .to_array()
                    })
                    .collect()
            } else {
                calculate_normals(&positions, &indices)
            };

            let material = primitive.material().pbr_metallic_roughness();

            let color = material.base_color_factor();
            let uv_channel = material
                .base_color_texture()
                .map(|t| t.tex_coord())
                .unwrap_or(0);

            let uvs: Vec<[f32; 2]> = if let Some(coords) = reader.read_tex_coords(uv_channel) {
                coords.into_f32().collect()
            } else {
                vec![[0.0, 0.0]; positions.len()]
            };

            let texture = material.base_color_texture().and_then(|info| {
                let index = info.texture().source().index();

                let data = images.get(index)?;

                match materials::save_texture(data, project) {
                    Ok(path) => Some(path),

                    Err(error) => {
                        eprintln!("[GXMaterial] {}", error);
                        None
                    }
                }
            });

            let pbr_info = material.metallic_roughness_texture();

            let pbr_channel = pbr_info.as_ref().map(|info| info.tex_coord()).unwrap_or(0);

            let pbr_uvs: Vec<[f32; 2]> = if let Some(coords) = reader.read_tex_coords(pbr_channel) {
                coords.into_f32().collect()
            } else {
                Vec::new()
            };

            let pbr_texture = pbr_info.and_then(|info| {
                if pbr_uvs.is_empty() {
                    return None;
                }

                let index = info.texture().source().index();

                let data = images.get(index)?;

                match materials::save_texture(data, project) {
                    Ok(path) => Some(path),

                    Err(error) => {
                        eprintln!("[GXMaterial] PBR import: {}", error);

                        None
                    }
                }
            });

            meshes.push(MeshData {
                uvs,
                texture,
                positions,
                normals,
                indices,
                color: [color[0], color[1], color[2]],
                metallic: material.metallic_factor(),
                roughness: material.roughness_factor(),
                pbr_texture,
                pbr_uvs,
            });
        }
    }

    for child in node.children() {
        import_node(child, world, buffers, images, project, meshes);
    }
}

fn calculate_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![Vec3::ZERO; positions.len()];

    for triangle in indices.chunks_exact(3) {
        let a = triangle[0] as usize;
        let b = triangle[1] as usize;
        let c = triangle[2] as usize;

        let p0 = Vec3::from_array(positions[a]);
        let p1 = Vec3::from_array(positions[b]);
        let p2 = Vec3::from_array(positions[c]);

        let normal = (p1 - p0).cross(p2 - p0);

        normals[a] += normal;
        normals[b] += normal;
        normals[c] += normal;
    }

    normals
        .into_iter()
        .map(|n| n.normalize_or_zero().to_array())
        .collect()
}

pub(super) fn load_meshes(
    path: &std::path::Path,
    project: &std::path::Path,
) -> Result<Vec<MeshData>, String> {
    let (document, buffers, images) = gltf::import(path).map_err(|e| e.to_string())?;

    let mut meshes = Vec::new();

    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next())
        .ok_or("Model contains no scenes")?;

    for node in scene.nodes() {
        import_node(
            node,
            glam::Mat4::IDENTITY,
            &buffers,
            &images,
            project,
            &mut meshes,
        );
    }

    if meshes.is_empty() {
        return Err("No triangle meshes found".into());
    }

    Ok(meshes)
}

impl GXEngine {
    pub(super) fn import_model(&mut self, path: &std::path::Path) {
        if self.playing {
            return;
        }

        match load_meshes(path, &self.project_dir) {
            Ok(meshes) => {
                if self.scene.objects.len() >= MAX_OBJECTS {
                    self.status = "Object limit reached".into();
                    return;
                }

                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();

                let triangles: usize = meshes.iter().map(|m| m.indices.len() / 3).sum();

                let mut object = Object::new(Shape::Mesh);

                object.name = name.clone();
                object.position = [0.0, 0.0, 0.0];
                object.scale = [1.0; 3];

                object.dynamic = false;
                object.color = [1.0; 3];

                object.meshes = std::sync::Arc::new(meshes);
                object.mesh_asset = None;

                self.scene.objects.push(object);

                self.selected = Some(self.scene.objects.len() - 1);

                self.environment_selected = false;
                self.save();

                self.status = format!("Imported {} | {} triangles", name, triangles);
            }

            Err(error) => {
                self.status = format!("Import failed: {}", error);
            }
        }
    }

    pub(super) fn mesh_vertices(&self) -> Vec<MeshVertex> {
        let texture_paths = self.mesh_texture_paths();
        let pbr_paths = self.mesh_pbr_paths();
        let mut vertices = Vec::new();

        for object in &self.scene.objects {
            if object.shape != Shape::Mesh {
                continue;
            }

            let rotation = Quat::from_euler(
                glam::EulerRot::XYZ,
                object.rotation[0].to_radians(),
                object.rotation[1].to_radians(),
                object.rotation[2].to_radians(),
            );

            let scale = Vec3::from_array(object.scale).max(Vec3::splat(0.05));

            let transform = glam::Mat4::from_scale_rotation_translation(
                scale,
                rotation,
                Vec3::from_array(object.position),
            );

            let normal_matrix = glam::Mat3::from_mat4(transform).inverse().transpose();

            for mesh in object.meshes.iter() {
                for &index in &mesh.indices {
                    let i = index as usize;

                    let position = transform.transform_point3(Vec3::from_array(mesh.positions[i]));

                    let normal =
                        (normal_matrix * Vec3::from_array(mesh.normals[i])).normalize_or_zero();

                    let color = Vec3::from_array(mesh.color) * Vec3::from_array(object.color);

                    vertices.push(MeshVertex {
                        position: position.to_array(),
                        normal: normal.to_array(),
                        color: color.to_array(),
                        material: [mesh.metallic, mesh.roughness],
                        uv: mesh.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                        pbr_uv: mesh.pbr_uvs.get(i).copied().unwrap_or([0.0, 0.0]),

                        pbr_layer: mesh
                            .pbr_texture
                            .as_ref()
                            .and_then(|texture| {
                                let path = self.project_dir.join(texture);

                                pbr_paths.iter().position(|p| p == &path)
                            })
                            .map(|index| index as f32)
                            .unwrap_or(-1.0),

                        texture_layer: mesh
                            .texture
                            .as_ref()
                            .and_then(|texture| {
                                let path = self.project_dir.join(texture);

                                texture_paths.iter().position(|p| p == &path)
                            })
                            .map(|index| index as f32)
                            .unwrap_or(-1.0),
                    });
                }
            }
        }

        vertices
    }

    pub(super) fn mesh_texture_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        for object in &self.scene.objects {
            for mesh in object.meshes.iter() {
                if let Some(texture) = &mesh.texture {
                    let path = self.project_dir.join(texture);

                    if !paths.contains(&path) && paths.len() < materials::MAX_TEXTURES {
                        paths.push(path);
                    }
                }
            }
        }

        paths
    }

    pub(super) fn mesh_pbr_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        for object in &self.scene.objects {
            for mesh in object.meshes.iter() {
                if let Some(texture) = &mesh.pbr_texture {
                    let path = self.project_dir.join(texture);

                    if !paths.contains(&path) && paths.len() < materials::MAX_TEXTURES {
                        paths.push(path);
                    }
                }
            }
        }

        paths
    }
}

impl MeshGPU {
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let textures = materials::MaterialTextures::new(device);
        let pbr_textures = materials::MaterialTextures::new_linear(device);
        let source = format!(
            "{}\n{}\n{}",
            include_str!("cinematic.wgsl"),
            include_str!("mesh_light.wgsl"),
            include_str!("mesh.wgsl"),
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GXMesh Shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GXMesh Layout"),
            bind_group_layouts: &[layout, &textures.layout, &pbr_textures.layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GXMesh Rasterizer"),

            layout: Some(&pipeline_layout),

            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("mesh_vertex"),

                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,

                    step_mode: wgpu::VertexStepMode::Vertex,

                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x3,
                        3 => Float32x2,
                        4 => Float32x2,
                        5 => Float32,
                        6 => Float32x2,
                        7 => Float32
                    ],
                }],

                compilation_options: Default::default(),
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("mesh_fragment"),

                targets: &[Some(wgpu::ColorTargetState {
                    format,

                    blend: Some(wgpu::BlendState::REPLACE),

                    write_mask: wgpu::ColorWrites::ALL,
                })],

                compilation_options: Default::default(),
            }),

            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },

            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,

                depth_write_enabled: true,

                depth_compare: wgpu::CompareFunction::LessEqual,

                stencil: Default::default(),
                bias: Default::default(),
            }),

            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        Self {
            textures,
            pbr_textures,
            pipeline,
            buffer: None,
            count: 0,
        }
    }

    pub(super) fn upload(&mut self, device: &wgpu::Device, vertices: &[MeshVertex]) {
        self.count = vertices.len() as u32;

        if vertices.is_empty() {
            self.buffer = None;
            return;
        }

        self.buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GXMesh Vertices"),

                contents: bytemuck::cast_slice(vertices),

                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
    }

    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'static>, bind_group: &wgpu::BindGroup) {
        if let Some(buffer) = &self.buffer {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_bind_group(1, &self.textures.bind_group, &[]);
            pass.set_bind_group(2, &self.pbr_textures.bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..self.count, 0..1);
        }
    }
}

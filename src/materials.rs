use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub(super) const TEXTURE_SIZE: u32 = 512;
pub(super) const MAX_TEXTURES: usize = 64;

// ==========================================
// IMPORT TEXTURE INTO PROJECT
// ==========================================

pub(super) fn save_texture(data: &gltf::image::Data, project: &Path) -> Result<String, String> {
    use gltf::image::Format;

    let mut rgba = Vec::new();

    match data.format {
        Format::R8 => {
            for &r in &data.pixels {
                rgba.extend_from_slice(&[r, r, r, 255]);
            }
        }

        Format::R8G8 => {
            for p in data.pixels.chunks_exact(2) {
                rgba.extend_from_slice(&[p[0], p[1], 0, 255]);
            }
        }

        Format::R8G8B8 => {
            for p in data.pixels.chunks_exact(3) {
                rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }

        Format::R8G8B8A8 => {
            rgba = data.pixels.clone();
        }

        _ => {
            return Err("Unsupported texture format".into());
        }
    }

    let mut hash = std::collections::hash_map::DefaultHasher::new();

    data.width.hash(&mut hash);
    data.height.hash(&mut hash);
    rgba.hash(&mut hash);

    let filename = format!("{:016x}.png", hash.finish());

    let relative = format!("Assets/Textures/{}", filename);

    let path = project.join(&relative);

    if !path.exists() {
        std::fs::create_dir_all(project.join("Assets/Textures")).map_err(|e| e.to_string())?;

        image::save_buffer(
            &path,
            &rgba,
            data.width,
            data.height,
            image::ColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(relative)
}

// ==========================================
// GPU TEXTURE ARRAY
// ==========================================

pub(super) struct MaterialTextures {
    format: wgpu::TextureFormat,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    paths: Vec<PathBuf>,
}

impl MaterialTextures {
    pub fn new(device: &wgpu::Device) -> Self {
        Self::new_with_format(device, wgpu::TextureFormat::Rgba8UnormSrgb)
    }

    pub fn new_linear(device: &wgpu::Device) -> Self {
        Self::new_with_format(device, wgpu::TextureFormat::Rgba8Unorm)
    }

    fn new_with_format(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GXMaterial Layout"),

            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,

                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },

                        view_dimension: wgpu::TextureViewDimension::D2Array,

                        multisampled: false,
                    },

                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,

                    visibility: wgpu::ShaderStages::FRAGMENT,

                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),

                    count: None,
                },
            ],
        });

        let texture = Self::create_texture(device, 1, format);

        let bind_group = Self::create_group(device, &layout, &texture);

        Self {
            layout,
            bind_group,
            paths: Vec::new(),
            format,
        }
    }

    fn create_texture(
        device: &wgpu::Device,
        layers: u32,
        format: wgpu::TextureFormat,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GXMaterial Textures"),

            size: wgpu::Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: layers,
            },

            mip_level_count: 1,
            sample_count: 1,

            dimension: wgpu::TextureDimension::D2,

            format,

            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,

            view_formats: &[],
        })
    }

    fn create_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture: &wgpu::Texture,
    ) -> wgpu::BindGroup {
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,

            address_mode_v: wgpu::AddressMode::Repeat,

            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,

            ..Default::default()
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GXMaterial Bind Group"),

            layout,

            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,

                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,

                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, paths: &[PathBuf]) {
        if paths == self.paths.as_slice() {
            return;
        }

        let texture = Self::create_texture(device, paths.len().max(1) as u32, self.format);

        for (layer, path) in paths.iter().enumerate() {
            let pixels = match image::open(path) {
                Ok(image) => image
                    .resize_exact(
                        TEXTURE_SIZE,
                        TEXTURE_SIZE,
                        image::imageops::FilterType::Triangle,
                    )
                    .to_rgba8()
                    .into_raw(),

                Err(error) => {
                    eprintln!("[GXMaterial] {}: {}", path.display(), error);

                    vec![255; (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize]
                }
            };

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,

                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },

                    aspect: wgpu::TextureAspect::All,
                },
                &pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(TEXTURE_SIZE * 4),
                    rows_per_image: Some(TEXTURE_SIZE),
                },
                wgpu::Extent3d {
                    width: TEXTURE_SIZE,
                    height: TEXTURE_SIZE,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.bind_group = Self::create_group(device, &self.layout, &texture);

        self.paths = paths.to_vec();
    }
}

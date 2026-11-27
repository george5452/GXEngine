use super::*;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const BUFFER_SIZE: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
struct Triangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
}

impl Triangle {
    fn center(&self) -> Vec3 {
        (self.a + self.b + self.c) / 3.0
    }
}

type Node = [[f32; 4]; 3];

pub(super) struct MeshLighting {
    pub buffer: wgpu::Buffer,
    previous_hash: Option<u64>,
}

fn build_bvh(triangles: &mut [Triangle], offset: usize, nodes: &mut Vec<Node>) -> usize {
    let id = nodes.len();

    nodes.push([[0.0; 4]; 3]);

    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);

    for triangle in triangles.iter() {
        lo = lo.min(triangle.a);
        lo = lo.min(triangle.b);
        lo = lo.min(triangle.c);

        hi = hi.max(triangle.a);
        hi = hi.max(triangle.b);
        hi = hi.max(triangle.c);
    }

    lo -= Vec3::splat(0.0001);
    hi += Vec3::splat(0.0001);

    let metadata;

    if triangles.len() <= 4 {
        metadata = [offset as f32, triangles.len() as f32, 1.0, 0.0];
    } else {
        let size = hi - lo;

        let axis = if size.x >= size.y && size.x >= size.z {
            0
        } else if size.y >= size.z {
            1
        } else {
            2
        };

        triangles.sort_unstable_by(|a, b| a.center()[axis].total_cmp(&b.center()[axis]));

        let middle = triangles.len() / 2;

        let (left, right) = triangles.split_at_mut(middle);

        let l = build_bvh(left, offset, nodes);

        let r = build_bvh(right, offset + middle, nodes);

        metadata = [l as f32, r as f32, 0.0, 0.0];
    }

    nodes[id] = [
        lo.extend(0.0).to_array(),
        hi.extend(0.0).to_array(),
        metadata,
    ];

    id
}

impl MeshLighting {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GX Mesh Lighting"),

            size: BUFFER_SIZE,

            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,

            mapped_at_creation: false,
        });

        Self {
            buffer,
            previous_hash: None,
        }
    }

    pub(super) fn upload(&mut self, queue: &wgpu::Queue, vertices: &[mesh::MeshVertex]) {
        let mut hasher = DefaultHasher::new();

        vertices.len().hash(&mut hasher);

        for vertex in vertices {
            for value in vertex.position {
                value.to_bits().hash(&mut hasher);
            }
        }

        let hash = hasher.finish();

        if self.previous_hash == Some(hash) {
            return;
        }

        self.previous_hash = Some(hash);

        let mut triangles = Vec::new();

        for v in vertices.chunks_exact(3) {
            triangles.push(Triangle {
                a: Vec3::from_array(v[0].position),
                b: Vec3::from_array(v[1].position),
                c: Vec3::from_array(v[2].position),
            });
        }

        let mut nodes = Vec::new();

        if !triangles.is_empty() {
            build_bvh(&mut triangles, 0, &mut nodes);
        }

        let triangle_offset = 1 + nodes.len() * 3;

        let mut data: Vec<[f32; 4]> = vec![[
            nodes.len() as f32,
            triangle_offset as f32,
            triangles.len() as f32,
            0.0,
        ]];

        for node in nodes {
            data.extend(node);
        }

        for triangle in triangles {
            data.push(triangle.a.extend(0.0).to_array());

            data.push(triangle.b.extend(0.0).to_array());

            data.push(triangle.c.extend(0.0).to_array());
        }

        let bytes: &[u8] = bytemuck::cast_slice(&data);

        if bytes.len() as u64 > BUFFER_SIZE {
            eprintln!("[GXLighting] Scene exceeds BVH capacity");

            queue.write_buffer(&self.buffer, 0, &[0u8; 16]);

            return;
        }

        queue.write_buffer(&self.buffer, 0, bytes);
    }
}

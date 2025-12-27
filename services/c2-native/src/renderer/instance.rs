use bytemuck::{Pod, Zeroable};

use crate::ecs::RenderInstance;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InstanceRaw {
    position: [f32; 3],
    size: f32,
    color: [f32; 4],
    heading: f32,
    kind: u32,
    _pad: [f32; 2],
}

impl InstanceRaw {
    pub fn from_instance(instance: &RenderInstance) -> Self {
        Self {
            position: instance.position.to_array(),
            size: instance.size,
            color: instance.color,
            heading: instance.heading_rad,
            kind: instance.kind,
            _pad: [0.0, 0.0],
        }
    }

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 36,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl Vertex {
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub fn quad_vertices() -> ([Vertex; 4], [u16; 6]) {
    let vertices = [
        Vertex {
            position: [-0.5, -0.5],
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [0.5, -0.5],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [0.5, 0.5],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [-0.5, 0.5],
            uv: [0.0, 0.0],
        },
    ];
    let indices = [0, 1, 2, 0, 2, 3];
    (vertices, indices)
}

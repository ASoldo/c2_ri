use glam::Vec3;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlobeVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv_equirect: [f32; 2],
    uv_merc: [f32; 2],
}

impl GlobeVertex {
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GlobeVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub fn build_sphere(radius: f32, segments: u32, rings: u32) -> (Vec<GlobeVertex>, Vec<u32>) {
    let segments = segments.max(3);
    let rings = rings.max(2);
    let mercator_sin_max = 85.051_128_78_f32.to_radians().sin();
    let mut vertices = Vec::with_capacity(((segments + 1) * (rings + 1)) as usize);
    let mut indices = Vec::with_capacity((segments * rings * 6) as usize);

    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let theta = v * std::f32::consts::PI;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        for segment in 0..=segments {
            let u = segment as f32 / segments as f32;
            let phi = u * std::f32::consts::TAU;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();
            let position = Vec3::new(
                radius * sin_theta * cos_phi,
                radius * cos_theta,
                radius * sin_theta * sin_phi,
            );
            let normal = position.normalize_or_zero();
            let sin_lat = (position.y / radius).clamp(-0.9999, 0.9999);
            let u_base = segment as f32 / segments as f32;
            let u = 1.5 - u_base;
            let v_equirect = v;
            let merc_sin = sin_lat.clamp(-mercator_sin_max, mercator_sin_max);
            let merc = 0.5
                - ((1.0 + merc_sin) / (1.0 - merc_sin)).ln()
                    / (4.0 * std::f32::consts::PI);
            vertices.push(GlobeVertex {
                position: position.to_array(),
                normal: normal.to_array(),
                uv_equirect: [u, v_equirect.clamp(0.0, 1.0)],
                uv_merc: [u, merc.clamp(0.0, 1.0)],
            });
        }
    }

    let stride = segments + 1;
    for ring in 0..rings {
        for segment in 0..segments {
            let i0 = ring * stride + segment;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;
            indices.push(i0);
            indices.push(i1);
            indices.push(i2);
            indices.push(i1);
            indices.push(i3);
            indices.push(i2);
        }
    }

    (vertices, indices)
}

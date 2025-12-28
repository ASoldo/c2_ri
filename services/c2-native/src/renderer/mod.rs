mod camera;
mod globe;
mod instance;
mod texture;

use wgpu::util::DeviceExt;

use crate::ecs::RenderInstance;
use crate::tiles::{TileKind, MAP_TILE_CAPACITY, SEA_TILE_CAPACITY, TILE_SIZE, WEATHER_TILE_CAPACITY};

pub use camera::{Camera, CameraController};
use globe::{build_sphere, GlobeVertex};
pub use instance::{quad_vertices, InstanceRaw, Vertex};
pub use texture::Texture;
use texture::{rgba_from_png, rgba_from_png_with_size, rgba_from_svg, TextureArray};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct OverlayUniform {
    base_opacity: f32,
    map_opacity: f32,
    weather_opacity: f32,
    sea_opacity: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TileUniform {
    radius: f32,
    opacity: f32,
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TileVertex {
    uv: [f32; 2],
}

impl TileVertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TileVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileInstanceRaw {
    pub bounds: [f32; 4],
    pub layer: f32,
}

impl TileInstanceRaw {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TileInstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    surface_format: wgpu::TextureFormat,
    size: (u32, u32),
    globe_pipeline: wgpu::RenderPipeline,
    globe_vertex_buffer: wgpu::Buffer,
    globe_index_buffer: wgpu::Buffer,
    globe_index_count: u32,
    globe_bind_group_layout: wgpu::BindGroupLayout,
    globe_bind_group: wgpu::BindGroup,
    marker_pipeline: wgpu::RenderPipeline,
    tile_pipeline: wgpu::RenderPipeline,
    tile_vertex_buffer: wgpu::Buffer,
    tile_index_buffer: wgpu::Buffer,
    tile_index_count: u32,
    tile_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    instance_count: u32,
    instance_raw: Vec<InstanceRaw>,
    camera: Camera,
    controller: CameraController,
    camera_buffer: wgpu::Buffer,
    marker_bind_group: wgpu::BindGroup,
    overlay_buffer: wgpu::Buffer,
    base_texture: Texture,
    map_texture: Texture,
    weather_texture: Texture,
    sea_texture: Texture,
    layer_size: u32,
    map_tiles: TileLayerGpu,
    weather_tiles: TileLayerGpu,
    sea_tiles: TileLayerGpu,
    viewport_texture: wgpu::Texture,
    viewport_view: wgpu::TextureView,
    viewport_depth: wgpu::Texture,
    viewport_depth_view: wgpu::TextureView,
    viewport_size: (u32, u32),
}

impl Renderer {
    pub async fn new(window: &winit::window::Window) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        // The surface must not outlive the window; the app owns the window for the event loop.
        let surface = unsafe {
            let surface = instance.create_surface(window)?;
            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("c2-native device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let config_size = (config.width, config.height);

        let camera = Camera::new(config.width as f32 / config.height as f32, 320.0);
        let controller = CameraController::new();
        let camera_uniform = CameraUniform {
            view_proj: camera.view_proj().to_cols_array_2d(),
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let overlay_uniform = OverlayUniform {
            base_opacity: 1.0,
            map_opacity: 0.85,
            weather_opacity: 0.55,
            sea_opacity: 0.45,
        };
        let overlay_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("overlay buffer"),
            contents: bytemuck::bytes_of(&overlay_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let (viewport_texture, viewport_view, viewport_depth, viewport_depth_view) =
            create_viewport_target(&device, surface_format, config.width, config.height);

        let layer_size = 4096u32;
        let (base_rgba, base_width, base_height) =
            rgba_from_png_with_size(include_bytes!("../../assets/earth_daymap.png"))
                .unwrap_or_else(|_| {
                    (
                        vec![32, 42, 68, 255].repeat((layer_size * layer_size) as usize),
                        layer_size,
                        layer_size,
                    )
                });
        let base_texture = Texture::from_rgba_globe(
            &device,
            &queue,
            base_width,
            base_height,
            &base_rgba,
            "base map",
        )
        .unwrap_or_else(|_| Texture::solid_rgba_globe(&device, &queue, [32, 42, 68, 255]));
        let map_texture = Texture::solid_rgba_globe(&device, &queue, [0, 0, 0, 0]);
        let weather_texture = Texture::solid_rgba_globe(&device, &queue, [0, 0, 0, 0]);
        let sea_texture = Texture::solid_rgba_globe(&device, &queue, [0, 0, 0, 0]);

        let globe_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("globe bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
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
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 9,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let globe_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globe bind group"),
            layout: &globe_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&base_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&base_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&map_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&map_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&weather_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&weather_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&sea_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&sea_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: overlay_buffer.as_entire_binding(),
                },
            ],
        });

        let icon_size = 128;
        let solid_layer = |color: [u8; 4]| {
            let mut data = Vec::with_capacity((icon_size * icon_size * 4) as usize);
            for _ in 0..(icon_size * icon_size) {
                data.extend_from_slice(&color);
            }
            data
        };
        let plane_layer = rgba_from_png(include_bytes!("../../assets/plane.png"), icon_size)
            .unwrap_or_else(|_| solid_layer([255, 255, 255, 255]));
        let ship_layer = rgba_from_svg(include_bytes!("../../assets/ship.svg"), icon_size)
            .unwrap_or_else(|_| solid_layer([34, 211, 238, 255]));
        let satellite_layer =
            rgba_from_svg(include_bytes!("../../assets/satellite.svg"), icon_size)
                .unwrap_or_else(|_| solid_layer([163, 230, 53, 255]));
        let icon_atlas = TextureArray::from_layers(
            &device,
            &queue,
            icon_size,
            icon_size,
            &[plane_layer, ship_layer, satellite_layer],
            "icon atlas",
        )
        .unwrap_or_else(|_| {
            let fallback = solid_layer([255, 255, 255, 255]);
            TextureArray::from_layers(
                &device,
                &queue,
                icon_size,
                icon_size,
                &[fallback.clone(), fallback.clone(), fallback],
                "icon atlas fallback",
            )
            .expect("fallback icon atlas")
        });

        let marker_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("marker bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
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
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let marker_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("marker bind group"),
            layout: &marker_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&icon_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&icon_atlas.sampler),
                },
            ],
        });

        let tile_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("tile bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let globe_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("globe shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/globe.wgsl").into(),
            ),
        });
        let globe_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("globe pipeline layout"),
                bind_group_layouts: &[&globe_bind_group_layout],
                immediate_size: 0,
            });
        let globe_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("globe pipeline"),
            layout: Some(&globe_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &globe_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[GlobeVertex::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &globe_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let marker_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("markers shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/markers.wgsl").into(),
            ),
        });

        let marker_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("marker pipeline layout"),
                bind_group_layouts: &[&marker_bind_group_layout],
                immediate_size: 0,
            });

        let marker_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("marker pipeline"),
            layout: Some(&marker_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &marker_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout(), InstanceRaw::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &marker_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let tile_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tile shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/tiles.wgsl").into(),
            ),
        });
        let tile_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("tile pipeline layout"),
                bind_group_layouts: &[&tile_bind_group_layout],
                immediate_size: 0,
            });
        let tile_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tile pipeline"),
            layout: Some(&tile_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &tile_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[TileVertex::layout(), TileInstanceRaw::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &tile_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let (globe_vertices, globe_indices) = build_sphere(120.0, 128, 64);
        let globe_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globe vertex buffer"),
            contents: bytemuck::cast_slice(&globe_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let globe_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globe index buffer"),
            contents: bytemuck::cast_slice(&globe_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let globe_index_count = globe_indices.len() as u32;

        let (vertices, indices) = quad_vertices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let (tile_vertices, tile_indices) = build_tile_mesh(16);
        let tile_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tile vertex buffer"),
            contents: bytemuck::cast_slice(&tile_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let tile_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tile index buffer"),
            contents: bytemuck::cast_slice(&tile_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let tile_index_count = tile_indices.len() as u32;

        let instance_capacity = 1024usize;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance buffer"),
            size: (instance_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let map_tiles = TileLayerGpu::new(
            &device,
            &queue,
            &tile_bind_group_layout,
            &camera_buffer,
            TILE_SIZE,
            MAP_TILE_CAPACITY as u32,
            120.0,
            1.0,
            "map tiles",
        )?;
        let weather_tiles = TileLayerGpu::new(
            &device,
            &queue,
            &tile_bind_group_layout,
            &camera_buffer,
            TILE_SIZE,
            WEATHER_TILE_CAPACITY as u32,
            120.0,
            0.55,
            "weather tiles",
        )?;
        let sea_tiles = TileLayerGpu::new(
            &device,
            &queue,
            &tile_bind_group_layout,
            &camera_buffer,
            TILE_SIZE,
            SEA_TILE_CAPACITY as u32,
            120.0,
            0.45,
            "sea tiles",
        )?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
            surface_format,
            size: config_size,
            globe_pipeline,
            globe_vertex_buffer,
            globe_index_buffer,
            globe_index_count,
            globe_bind_group_layout,
            globe_bind_group,
            marker_pipeline,
            tile_pipeline,
            tile_vertex_buffer,
            tile_index_buffer,
            tile_index_count,
            tile_bind_group_layout,
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            instance_buffer,
            instance_capacity,
            instance_count: 0,
            instance_raw: Vec::new(),
            camera,
            controller,
            camera_buffer,
            marker_bind_group,
            overlay_buffer,
            base_texture,
            map_texture,
            weather_texture,
            sea_texture,
            layer_size,
            map_tiles,
            weather_tiles,
            sea_tiles,
            viewport_texture,
            viewport_view,
            viewport_depth,
            viewport_depth_view,
            viewport_size: config_size,
        })
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn viewport_size(&self) -> (u32, u32) {
        self.viewport_size
    }

    pub fn layer_size(&self) -> u32 {
        self.layer_size
    }

    pub fn camera_distance(&self) -> f32 {
        self.camera.distance
    }

    pub fn camera_position(&self) -> glam::Vec3 {
        self.camera.position()
    }

    pub fn camera_fov_y(&self) -> f32 {
        self.camera.fov_y
    }

    pub fn camera_aspect(&self) -> f32 {
        self.camera.aspect
    }

    pub fn viewport_view(&self) -> &wgpu::TextureView {
        &self.viewport_view
    }

    pub fn viewport_depth_view(&self) -> &wgpu::TextureView {
        &self.viewport_depth_view
    }

    pub fn create_window_surface(
        &self,
        window: &winit::window::Window,
    ) -> anyhow::Result<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration)> {
        let surface = unsafe {
            let surface = self.instance.create_surface(window)?;
            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
        };
        let caps = surface.get_capabilities(&self.adapter);
        let surface_format = if caps.formats.contains(&self.surface_format) {
            self.surface_format
        } else {
            caps.formats
                .iter()
                .copied()
                .find(|format| format.is_srgb())
                .unwrap_or(caps.formats[0])
        };
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&self.device, &config);
        Ok((surface, config))
    }

    pub fn ensure_viewport_size(&mut self, width: u32, height: u32) -> bool {
        let width = width.max(1);
        let height = height.max(1);
        if (width, height) == self.viewport_size {
            return false;
        }
        let (texture, view, depth, depth_view) =
            create_viewport_target(&self.device, self.surface_format, width, height);
        self.viewport_texture = texture;
        self.viewport_view = view;
        self.viewport_depth = depth;
        self.viewport_depth_view = depth_view;
        self.viewport_size = (width, height);
        self.camera.update_aspect(width, height);
        true
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.size = (width, height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.camera.update_aspect(width, height);
    }

    pub fn handle_input(&mut self, event: &winit::event::WindowEvent) {
        self.controller.process_event(event, &mut self.camera);
    }

    pub fn update_instances(&mut self, instances: &[RenderInstance]) {
        let needed = instances.len().max(1).next_power_of_two();
        if needed > self.instance_capacity {
            self.instance_capacity = needed;
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instance buffer"),
                size: (self.instance_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.instance_raw.clear();
        self.instance_raw
            .extend(instances.iter().map(InstanceRaw::from_instance));
        self.queue
            .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&self.instance_raw));
        self.instance_count = self.instance_raw.len() as u32;
    }

    pub fn begin_frame(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    pub fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    fn update_camera(&self) {
        let uniform = CameraUniform {
            view_proj: self.camera.view_proj().to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn view_proj(&self) -> glam::Mat4 {
        self.camera.view_proj()
    }

    pub fn render_scene(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.update_camera();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("globe + marker pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.viewport_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.04,
                        g: 0.05,
                        b: 0.07,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.viewport_depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.globe_pipeline);
        pass.set_bind_group(0, &self.globe_bind_group, &[]);
        pass.set_vertex_buffer(0, self.globe_vertex_buffer.slice(..));
        pass.set_index_buffer(self.globe_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.globe_index_count, 0, 0..1);

        self.render_tile_layer(&mut pass, &self.map_tiles);
        self.render_tile_layer(&mut pass, &self.sea_tiles);
        self.render_tile_layer(&mut pass, &self.weather_tiles);

        pass.set_pipeline(&self.marker_pipeline);
        pass.set_bind_group(0, &self.marker_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.num_indices, 0, 0..self.instance_count);
    }

    fn render_tile_layer<'a>(
        &self,
        pass: &mut wgpu::RenderPass<'a>,
        layer: &'a TileLayerGpu,
    ) {
        if layer.instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.tile_pipeline);
        pass.set_bind_group(0, &layer.bind_group, &[]);
        pass.set_vertex_buffer(0, self.tile_vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, layer.instance_buffer.slice(..));
        pass.set_index_buffer(self.tile_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.tile_index_count, 0, 0..layer.instance_count);
    }

    pub fn orbit_delta(&mut self, dx: f32, dy: f32) {
        self.controller.orbit_delta(dx, dy, &mut self.camera);
    }

    pub fn zoom_delta(&mut self, scroll: f32) {
        self.controller.zoom_delta(scroll, &mut self.camera);
    }

    pub fn update_layer(&mut self, kind: GlobeLayer, width: u32, height: u32, data: &[u8]) {
        let label = match kind {
            GlobeLayer::Base => "base map layer",
            GlobeLayer::Map => "map tiles layer",
            GlobeLayer::Weather => "weather layer",
            GlobeLayer::Sea => "sea layer",
        };
        if let Ok(texture) =
            Texture::from_rgba_globe(&self.device, &self.queue, width, height, data, label)
        {
            match kind {
                GlobeLayer::Base => self.base_texture = texture,
                GlobeLayer::Map => self.map_texture = texture,
                GlobeLayer::Weather => self.weather_texture = texture,
                GlobeLayer::Sea => self.sea_texture = texture,
            }
            self.layer_size = width;
            self.rebuild_globe_bind_group();
        }
    }

    pub fn update_tile_instances(&mut self, kind: TileKind, instances: &[TileInstanceRaw]) {
        let device = self.device.clone();
        let queue = self.queue.clone();
        let layer = self.tile_layer_mut(kind);
        let needed = instances.len().max(1).next_power_of_two();
        if needed > layer.instance_capacity {
            layer.instance_capacity = needed;
            layer.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tile instance buffer"),
                size: (layer.instance_capacity * std::mem::size_of::<TileInstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        layer.instances.clear();
        layer.instances.extend_from_slice(instances);
        if !layer.instances.is_empty() {
            queue.write_buffer(
                &layer.instance_buffer,
                0,
                bytemuck::cast_slice(&layer.instances),
            );
        }
        layer.instance_count = layer.instances.len() as u32;
    }

    pub fn update_tile_texture(
        &mut self,
        kind: TileKind,
        layer_index: u32,
        width: u32,
        height: u32,
        data: &[u8],
    ) {
        let queue = self.queue.clone();
        let layer = self.tile_layer_mut(kind);
        let width = width.min(layer.atlas.width);
        let height = height.min(layer.atlas.height);
        if width == 0 || height == 0 {
            return;
        }
        if layer_index >= layer.atlas.layers {
            return;
        }
        let expected = (width * height * 4) as usize;
        if data.len() < expected {
            return;
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &layer.atlas.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer_index,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &data[..expected],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn update_tile_opacity(&mut self, kind: TileKind, opacity: f32) {
        let queue = self.queue.clone();
        let layer = self.tile_layer_mut(kind);
        let uniform = TileUniform {
            radius: layer.radius,
            opacity: opacity.clamp(0.0, 1.0),
            _pad: [0.0; 2],
        };
        queue.write_buffer(&layer.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn update_overlay(&self, base: f32, map: f32, sea: f32, weather: f32) {
        let uniform = OverlayUniform {
            base_opacity: base,
            map_opacity: map,
            weather_opacity: weather,
            sea_opacity: sea,
        };
        self.queue
            .write_buffer(&self.overlay_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    fn rebuild_globe_bind_group(&mut self) {
        self.globe_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globe bind group"),
            layout: &self.globe_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.base_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.base_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.map_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.map_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&self.weather_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.weather_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&self.sea_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.sea_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: self.overlay_buffer.as_entire_binding(),
                },
            ],
        });
    }

    fn tile_layer_mut(&mut self, kind: TileKind) -> &mut TileLayerGpu {
        match kind {
            TileKind::Base => &mut self.map_tiles,
            TileKind::Weather => &mut self.weather_tiles,
            TileKind::Sea => &mut self.sea_tiles,
        }
    }

    fn tile_layer(&self, kind: TileKind) -> &TileLayerGpu {
        match kind {
            TileKind::Base => &self.map_tiles,
            TileKind::Weather => &self.weather_tiles,
            TileKind::Sea => &self.sea_tiles,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobeLayer {
    Base,
    Map,
    Weather,
    Sea,
}

struct TileLayerGpu {
    atlas: TextureArray,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    instance_count: u32,
    instances: Vec<TileInstanceRaw>,
    radius: f32,
}

impl TileLayerGpu {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        tile_size: u32,
        capacity: u32,
        radius: f32,
        opacity: f32,
        label: &str,
    ) -> anyhow::Result<Self> {
        let atlas = TextureArray::empty(device, queue, tile_size, tile_size, capacity, label)?;
        let uniform = TileUniform {
            radius,
            opacity: opacity.clamp(0.0, 1.0),
            _pad: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{label} uniform")),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{label} bind group")),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&atlas.sampler),
                },
            ],
        });
        let instance_capacity = 256usize;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label} instance buffer")),
            size: (instance_capacity * std::mem::size_of::<TileInstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            atlas,
            uniform_buffer,
            bind_group,
            instance_buffer,
            instance_capacity,
            instance_count: 0,
            instances: Vec::new(),
            radius,
        })
    }
}

fn create_viewport_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> (
    wgpu::Texture,
    wgpu::TextureView,
    wgpu::Texture,
    wgpu::TextureView,
) {
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("viewport color"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());

    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("viewport depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    (color, color_view, depth, depth_view)
}

fn build_tile_mesh(segments: u32) -> (Vec<TileVertex>, Vec<u16>) {
    let segments = segments.max(2);
    let stride = segments + 1;
    let mut vertices = Vec::with_capacity((stride * stride) as usize);
    for y in 0..=segments {
        let v = y as f32 / segments as f32;
        for x in 0..=segments {
            let u = x as f32 / segments as f32;
            vertices.push(TileVertex { uv: [u, v] });
        }
    }
    let mut indices = Vec::with_capacity((segments * segments * 6) as usize);
    for y in 0..segments {
        for x in 0..segments {
            let i0 = (y * stride + x) as u16;
            let i1 = i0 + 1;
            let i2 = i0 + stride as u16;
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

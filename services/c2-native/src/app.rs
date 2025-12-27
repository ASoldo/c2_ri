use std::sync::Arc;
use std::time::Instant;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes};

use crate::ecs::{RenderInstance, WorldState, KIND_FLIGHT, KIND_SATELLITE, KIND_SHIP};
use crate::renderer::{GlobeLayer, Renderer};
use crate::tiles::{TileFetcher, TileKind, TileRequest, TileResult};
use crate::ui::{tile_provider_config, OperationsState, TileProviderConfig, UiState};

const DEFAULT_GLOBE_RADIUS: f32 = 120.0;
const TILE_ZOOM_CAP: u8 = 6;
const MAP_MAX_LAYER_SIZE: u32 = 8192;
const OVERLAY_MAX_LAYER_SIZE: u32 = 4096;
const WEATHER_MIN_ZOOM: u8 = 0;
const WEATHER_MAX_ZOOM: u8 = 6;
const SEA_MIN_ZOOM: u8 = 0;
const SEA_MAX_ZOOM: u8 = 6;

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let window = Arc::new(event_loop.create_window(
        WindowAttributes::default().with_title("C2 Walaris"),
    )?);
    let target_window_id = window.id();
    let mut app = App::new(window.clone())?;

    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent { event, window_id } if window_id == target_window_id => {
                if let WindowEvent::RedrawRequested = event {
                    if let Err(error) = app.update_and_render() {
                        eprintln!("render error: {error:?}");
                    }
                    return;
                }
                if !app.handle_window_event(&event) {
                    target.exit();
                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    })?;

    Ok(())
}

struct App {
    window: Arc<Window>,
    renderer: Renderer,
    world: WorldState,
    ui: UiState,
    instances: Vec<RenderInstance>,
    filtered_instances: Vec<RenderInstance>,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    last_frame: Instant,
    globe_texture_id: Option<egui::TextureId>,
    globe_dragging: bool,
    overlay_settings: OperationsState,
    world_accum: f32,
    world_update_interval: f32,
    instances_dirty: bool,
    tile_fetcher: TileFetcher,
    tile_rx: std::sync::mpsc::Receiver<TileResult>,
    tile_zoom_map: u8,
    tile_zoom_weather: u8,
    tile_zoom_sea: u8,
    tile_request_id: u64,
    tile_pending: Option<TilePending>,
    tile_settings: TileSettings,
}

impl App {
    fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let renderer = pollster::block_on(Renderer::new(window.as_ref()))?;
        let world = WorldState::seeded();
        let ui = UiState::new();

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &renderer.device,
            renderer.surface_format(),
            egui_wgpu::RendererOptions::default(),
        );
        let overlay_settings = ui.operations().clone();
        renderer.update_overlay(
            if overlay_settings.show_base { 1.0 } else { 0.0 },
            if overlay_settings.show_map { 0.85 } else { 0.0 },
            if overlay_settings.show_sea { 0.45 } else { 0.0 },
            if overlay_settings.show_weather { 0.55 } else { 0.0 },
        );
        let tile_settings = TileSettings::from(&overlay_settings);
        let (tile_fetcher, tile_rx) = TileFetcher::new();
        let provider = tile_provider_config(&overlay_settings.tile_provider);
        let tile_zoom_map = pick_tile_zoom(&renderer, provider, DEFAULT_GLOBE_RADIUS);
        let map_target_size = target_layer_size(tile_zoom_map, MAP_MAX_LAYER_SIZE);
        let tile_zoom_weather =
            pick_overlay_zoom(&renderer, WEATHER_MIN_ZOOM, WEATHER_MAX_ZOOM, DEFAULT_GLOBE_RADIUS);
        let weather_target_size = target_layer_size(tile_zoom_weather, OVERLAY_MAX_LAYER_SIZE);
        let tile_zoom_sea =
            pick_overlay_zoom(&renderer, SEA_MIN_ZOOM, SEA_MAX_ZOOM, DEFAULT_GLOBE_RADIUS);
        let sea_target_size = target_layer_size(tile_zoom_sea, OVERLAY_MAX_LAYER_SIZE);
        let tile_request_id = 1;
        let mut pending = std::collections::HashSet::new();
        let base_request = TileRequest {
            request_id: tile_request_id,
            zoom: tile_zoom_map,
            provider: tile_settings.provider.clone(),
            weather_field: tile_settings.weather_field.clone(),
            sea_field: tile_settings.sea_field.clone(),
            target_size: map_target_size,
        };
        pending.insert(TileKind::Base);
        tile_fetcher.request(TileKind::Base, base_request);

        if overlay_settings.show_weather {
            pending.insert(TileKind::Weather);
            tile_fetcher.request(
                TileKind::Weather,
                TileRequest {
                    request_id: tile_request_id,
                    zoom: tile_zoom_weather,
                    provider: tile_settings.provider.clone(),
                    weather_field: tile_settings.weather_field.clone(),
                    sea_field: tile_settings.sea_field.clone(),
                    target_size: weather_target_size,
                },
            );
        }
        if overlay_settings.show_sea {
            pending.insert(TileKind::Sea);
            tile_fetcher.request(
                TileKind::Sea,
                TileRequest {
                    request_id: tile_request_id,
                    zoom: tile_zoom_sea,
                    provider: tile_settings.provider.clone(),
                    weather_field: tile_settings.weather_field.clone(),
                    sea_field: tile_settings.sea_field.clone(),
                    target_size: sea_target_size,
                },
            );
        }

        let pending_total = pending.len();
        Ok(Self {
            window,
            renderer,
            world,
            ui,
            instances: Vec::new(),
            filtered_instances: Vec::new(),
            egui_ctx,
            egui_state,
            egui_renderer,
            last_frame: Instant::now(),
            globe_texture_id: None,
            globe_dragging: false,
            overlay_settings,
            world_accum: 1.0 / 30.0,
            world_update_interval: 1.0 / 30.0,
            instances_dirty: true,
            tile_fetcher,
            tile_rx,
            tile_zoom_map,
            tile_zoom_weather,
            tile_zoom_sea,
            tile_request_id,
            tile_pending: Some(TilePending {
                request_id: tile_request_id,
                pending,
                total: pending_total,
            }),
            tile_settings,
        })
    }

    fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        let window = self.window.as_ref();
        let egui_response = self.egui_state.on_window_event(window, event);
        if !egui_response.consumed {
            self.renderer.handle_input(event);
        }
        match event {
            WindowEvent::CloseRequested => return false,
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = window.inner_size();
                self.renderer.resize(size.width, size.height);
            }
            _ => {}
        }
        true
    }

    fn update_and_render(&mut self) -> anyhow::Result<()> {
        let window = self.window.as_ref();
        let now = Instant::now();
        let delta = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        let mut world_updated = false;
        self.world_accum += delta.max(0.0);
        if self.world_accum >= self.world_update_interval {
            let steps = (self.world_accum / self.world_update_interval)
                .floor()
                .min(4.0) as u32;
            for _ in 0..steps {
                self.world.update(self.world_update_interval);
            }
            self.world_accum -= steps as f32 * self.world_update_interval;
            world_updated = true;
            self.instances_dirty = true;
        }
        if world_updated || self.instances_dirty {
            self.world.collect_instances(&mut self.instances);
        }
        if self.instances_dirty {
            filter_instances(
                &self.instances,
                &self.overlay_settings,
                &mut self.filtered_instances,
            );
            self.renderer.update_instances(&self.filtered_instances);
            self.instances_dirty = false;
        }

        let raw_input = self.egui_state.take_egui_input(window);
        let tiles_loading = self
            .tile_pending
            .as_ref()
            .map(|pending| {
                if pending.total == 0 {
                    0.0
                } else {
                    (pending.total.saturating_sub(pending.pending.len())) as f32
                        / pending.total as f32
                }
            });

        let output = self.egui_ctx.run(raw_input, |ctx| {
            self.ui.show(
                ctx,
                &self.world,
                &self.renderer,
                &self.filtered_instances,
                self.globe_texture_id,
                tiles_loading,
            );
        });
        self.egui_state
            .handle_platform_output(window, output.platform_output);
        let paint_jobs = self.egui_ctx.tessellate(output.shapes, output.pixels_per_point);

        if let Some(rect) = self.ui.globe_rect() {
            self.egui_ctx.input(|input| {
                if let Some(pos) = input.pointer.latest_pos() {
                    let hovered = rect.contains(pos);
                    if input.pointer.primary_pressed() && hovered {
                        self.globe_dragging = true;
                    }
                    if input.pointer.primary_released() {
                        self.globe_dragging = false;
                    }
                    if self.globe_dragging && input.pointer.primary_down() {
                        let delta = input.pointer.delta();
                        if delta.x.abs() > 0.0 || delta.y.abs() > 0.0 {
                            self.renderer.orbit_delta(delta.x, delta.y);
                        }
                    }
                    if hovered {
                        let scroll = input.smooth_scroll_delta.y;
                        if scroll.abs() > 0.0 {
                            self.renderer.zoom_delta(scroll);
                        }
                    }
                } else if input.pointer.primary_released() {
                    self.globe_dragging = false;
                }
            });
        }

        let new_settings = self.ui.operations().clone();
        if new_settings != self.overlay_settings {
            self.overlay_settings = new_settings;
            filter_instances(
                &self.instances,
                &self.overlay_settings,
                &mut self.filtered_instances,
            );
            self.renderer.update_instances(&self.filtered_instances);
            self.instances_dirty = false;
            self.renderer.update_overlay(
                if self.overlay_settings.show_base { 1.0 } else { 0.0 },
                if self.overlay_settings.show_map { 0.85 } else { 0.0 },
                if self.overlay_settings.show_sea { 0.45 } else { 0.0 },
                if self.overlay_settings.show_weather { 0.55 } else { 0.0 },
            );
        }

        for (id, image_delta) in &output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.renderer.device, &self.renderer.queue, *id, image_delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.renderer.size().0, self.renderer.size().1],
            pixels_per_point: output.pixels_per_point,
        };

        if let Some(rect) = self.ui.globe_rect() {
            let width = (rect.width() * output.pixels_per_point).round() as u32;
            let height = (rect.height() * output.pixels_per_point).round() as u32;
            let resized = self.renderer.ensure_viewport_size(width, height);
            if resized || self.globe_texture_id.is_none() {
                if let Some(texture_id) = self.globe_texture_id {
                    self.egui_renderer.update_egui_texture_from_wgpu_texture(
                        &self.renderer.device,
                        self.renderer.viewport_view(),
                        wgpu::FilterMode::Linear,
                        texture_id,
                    );
                } else {
                    self.globe_texture_id = Some(self.egui_renderer.register_native_texture(
                        &self.renderer.device,
                        self.renderer.viewport_view(),
                        wgpu::FilterMode::Linear,
                    ));
                }
            }
        }

        let provider = tile_provider_config(&self.overlay_settings.tile_provider);
        let desired_map_zoom = pick_tile_zoom(&self.renderer, provider, DEFAULT_GLOBE_RADIUS);
        let desired_weather_zoom =
            pick_overlay_zoom(&self.renderer, WEATHER_MIN_ZOOM, WEATHER_MAX_ZOOM, DEFAULT_GLOBE_RADIUS);
        let desired_sea_zoom =
            pick_overlay_zoom(&self.renderer, SEA_MIN_ZOOM, SEA_MAX_ZOOM, DEFAULT_GLOBE_RADIUS);
        let desired_map_size = target_layer_size(desired_map_zoom, MAP_MAX_LAYER_SIZE);
        let desired_weather_size = target_layer_size(desired_weather_zoom, OVERLAY_MAX_LAYER_SIZE);
        let desired_sea_size = target_layer_size(desired_sea_zoom, OVERLAY_MAX_LAYER_SIZE);
        let new_tile_settings = TileSettings::from(&self.overlay_settings);
        let tile_settings_changed = new_tile_settings != self.tile_settings;
        let map_request =
            self.overlay_settings.show_map
                && (tile_settings_changed || desired_map_zoom != self.tile_zoom_map);
        let weather_request = self.overlay_settings.show_weather
            && (tile_settings_changed || desired_weather_zoom != self.tile_zoom_weather);
        let sea_request =
            self.overlay_settings.show_sea
                && (tile_settings_changed || desired_sea_zoom != self.tile_zoom_sea);

        if map_request || weather_request || sea_request {
            self.tile_request_id += 1;
            let mut pending = std::collections::HashSet::new();
            if map_request {
                pending.insert(TileKind::Base);
                self.tile_fetcher.request(
                    TileKind::Base,
                    TileRequest {
                        request_id: self.tile_request_id,
                        zoom: desired_map_zoom,
                        provider: new_tile_settings.provider.clone(),
                        weather_field: new_tile_settings.weather_field.clone(),
                        sea_field: new_tile_settings.sea_field.clone(),
                        target_size: desired_map_size,
                    },
                );
                self.tile_zoom_map = desired_map_zoom;
            }
            if weather_request {
                pending.insert(TileKind::Weather);
                self.tile_fetcher.request(
                    TileKind::Weather,
                    TileRequest {
                        request_id: self.tile_request_id,
                        zoom: desired_weather_zoom,
                        provider: new_tile_settings.provider.clone(),
                        weather_field: new_tile_settings.weather_field.clone(),
                        sea_field: new_tile_settings.sea_field.clone(),
                        target_size: desired_weather_size,
                    },
                );
                self.tile_zoom_weather = desired_weather_zoom;
            }
            if sea_request {
                pending.insert(TileKind::Sea);
                self.tile_fetcher.request(
                    TileKind::Sea,
                    TileRequest {
                        request_id: self.tile_request_id,
                        zoom: desired_sea_zoom,
                        provider: new_tile_settings.provider.clone(),
                        weather_field: new_tile_settings.weather_field.clone(),
                        sea_field: new_tile_settings.sea_field.clone(),
                        target_size: desired_sea_size,
                    },
                );
                self.tile_zoom_sea = desired_sea_zoom;
            }
            if pending.is_empty() {
                self.tile_pending = None;
            } else {
                let total = pending.len();
                self.tile_pending = Some(TilePending {
                    request_id: self.tile_request_id,
                    pending,
                    total,
                });
            }
            self.tile_settings = new_tile_settings;
        }
        if let Some(pending) = &mut self.tile_pending {
            for result in self.tile_rx.try_iter() {
                if result.request_id != pending.request_id {
                    continue;
                }
                apply_tile_result(&mut self.renderer, &result);
                pending.pending.remove(&result.kind);
                if pending.pending.is_empty() {
                    self.tile_pending = None;
                    break;
                }
            }
        }

        let mut encoder = self
            .renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("c2-native encoder"),
            });

        self.renderer.render_scene(&mut encoder);

        let mut egui_cmds = self.egui_renderer.update_buffers(
            &self.renderer.device,
            &self.renderer.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let surface_texture = match self.renderer.begin_frame() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                self.renderer.reconfigure();
                return Ok(());
            }
            Err(wgpu::SurfaceError::Lost) => {
                self.renderer.reconfigure();
                return Ok(());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(anyhow::anyhow!("surface out of memory"));
            }
            Err(err) => {
                return Err(anyhow::anyhow!("surface error: {err:?}"));
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let egui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut egui_pass = egui_pass.forget_lifetime();
            self.egui_renderer
                .render(&mut egui_pass, &paint_jobs, &screen_descriptor);
        }

        egui_cmds.push(encoder.finish());
        self.renderer.queue.submit(egui_cmds);
        surface_texture.present();

        for id in &output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        Ok(())
    }
}

fn apply_tile_result(renderer: &mut Renderer, result: &TileResult) {
    if !result.valid {
        return;
    }
    let layer = match result.kind {
        TileKind::Base => GlobeLayer::Map,
        TileKind::Weather => GlobeLayer::Weather,
        TileKind::Sea => GlobeLayer::Sea,
    };
    renderer.update_layer(layer, result.width, result.height, &result.data);
}

#[derive(Debug, Clone, PartialEq)]
struct TileSettings {
    provider: String,
    weather_field: String,
    sea_field: String,
    show_map: bool,
    show_weather: bool,
    show_sea: bool,
}

impl From<&OperationsState> for TileSettings {
    fn from(settings: &OperationsState) -> Self {
        Self {
            provider: settings.tile_provider.clone(),
            weather_field: settings.weather_field.clone(),
            sea_field: settings.sea_field.clone(),
            show_map: settings.show_map,
            show_weather: settings.show_weather,
            show_sea: settings.show_sea,
        }
    }
}

struct TilePending {
    request_id: u64,
    pending: std::collections::HashSet<TileKind>,
    total: usize,
}

fn filter_instances(
    instances: &[RenderInstance],
    settings: &OperationsState,
    out: &mut Vec<RenderInstance>,
) {
    out.clear();
    for instance in instances {
        match instance.category {
            KIND_FLIGHT if !settings.show_flights => continue,
            KIND_SHIP if !settings.show_ships => continue,
            KIND_SATELLITE if !settings.show_satellites => continue,
            _ => {}
        }
        out.push(*instance);
    }
}

fn pick_tile_zoom(renderer: &Renderer, provider: TileProviderConfig, globe_radius: f32) -> u8 {
    pick_zoom(renderer, globe_radius, provider.min_zoom, provider.max_zoom, provider.zoom_bias)
}

fn pick_overlay_zoom(
    renderer: &Renderer,
    min_zoom: u8,
    max_zoom: u8,
    globe_radius: f32,
) -> u8 {
    pick_zoom(renderer, globe_radius, min_zoom, max_zoom, 0)
}

fn pick_zoom(
    renderer: &Renderer,
    globe_radius: f32,
    min_zoom: u8,
    max_zoom: u8,
    zoom_bias: i8,
) -> u8 {
    let (width, height) = renderer.viewport_size();
    if width == 0 || height == 0 {
        return min_zoom;
    }
    let distance = renderer.camera_distance();
    let depth = (distance - globe_radius).max(1.0);
    let fov_v = renderer.camera_fov_y();
    let aspect = renderer.camera_aspect();
    let fov_h = 2.0 * ((fov_v * 0.5).tan() * aspect).atan();
    let visible_width = 2.0 * depth * (fov_h * 0.5).tan();
    let visible_height = 2.0 * depth * (fov_v * 0.5).tan();
    let deg_width = (visible_width / globe_radius) * (180.0 / std::f32::consts::PI);
    let deg_height = (visible_height / globe_radius) * (180.0 / std::f32::consts::PI);
    let tile_deg_width = deg_width * (256.0 / width as f32);
    let tile_deg_height = deg_height * (256.0 / height as f32);
    let tile_deg = tile_deg_width.max(tile_deg_height).max(0.0001);
    let mut zoom = (360.0 / tile_deg).log2().round() as i32 + zoom_bias as i32;
    let max_zoom = max_zoom.min(TILE_ZOOM_CAP) as i32;
    let min_zoom = min_zoom as i32;
    zoom = zoom.clamp(min_zoom, max_zoom);
    zoom as u8
}

fn target_layer_size(zoom: u8, max_size: u32) -> u32 {
    let tiles = 1u32 << zoom;
    let mosaic = tiles.saturating_mul(256);
    mosaic.clamp(512, max_size)
}

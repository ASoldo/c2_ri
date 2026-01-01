use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use egui_dock::DockState;
use glam::Vec3;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::ecs::{RenderInstance, WorldState, KIND_FLIGHT, KIND_SATELLITE, KIND_SHIP};
use crate::renderer::{Renderer, TileInstanceRaw};
use crate::tiles::{
    TileFetcher, TileKey, TileKind, TileRequest, TileResult, MAP_TILE_CAPACITY,
    SEA_TILE_CAPACITY, TILE_SIZE, WEATHER_TILE_CAPACITY,
};
use crate::ui::{
    tile_provider_config, Diagnostics, DockDetachRequest, DockDragStart, DockHost, DockTab,
    OperationsState, PerfSnapshot, TileProviderConfig, UiState,
};

const DEFAULT_GLOBE_RADIUS: f32 = 120.0;
const TILE_ZOOM_CAP: u8 = 6;
const WEATHER_MIN_ZOOM: u8 = 0;
const WEATHER_MAX_ZOOM: u8 = 6;
const SEA_MIN_ZOOM: u8 = 0;
const SEA_MAX_ZOOM: u8 = 6;
const MAP_UPDATE_INTERVAL_MS: u64 = 220;
const WEATHER_UPDATE_INTERVAL_MS: u64 = 900;
const SEA_UPDATE_INTERVAL_MS: u64 = 1100;
const MAX_TILE_UPLOADS_PER_FRAME: usize = 24;
const TILE_STALL_THRESHOLD_MS: f32 = 8000.0;
const PERF_SAMPLE_COUNT: usize = 120;

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let window = Arc::new(event_loop.create_window(
        WindowAttributes::default().with_title("C2 Walaris"),
    )?);
    let mut app = App::new(window.clone())?;

    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent { event, window_id } => {
                if window_id == app.main_window_id {
                    if let WindowEvent::RedrawRequested = event {
                        if let Err(error) = app.update_and_render() {
                            eprintln!("render error: {error:?}");
                        }
                        app.process_ui_requests(target);
                        return;
                    }
                    if !app.handle_window_event(&event) {
                        target.exit();
                    }
                } else if app.is_detached_window(window_id) {
                    if let WindowEvent::RedrawRequested = event {
                        if let Err(error) = app.render_detached_window(window_id) {
                            eprintln!("detach render error: {error:?}");
                        }
                        app.sync_operations_settings();
                        app.process_ui_requests(target);
                        return;
                    }
                    app.handle_detached_window_event(window_id, &event);
                }
            }
            Event::AboutToWait => {
                app.request_redraws();
            }
            _ => {}
        }
    })?;

    Ok(())
}

struct App {
    window: Arc<Window>,
    main_window_id: WindowId,
    renderer: Renderer,
    world: WorldState,
    ui: UiState,
    instances: Vec<RenderInstance>,
    filtered_instances: Vec<RenderInstance>,
    render_instances: Vec<RenderInstance>,
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
    cull_dirty: bool,
    tile_fetcher: TileFetcher,
    tile_rx: std::sync::mpsc::Receiver<TileResult>,
    tile_request_id: u64,
    tile_layers: TileLayers,
    diagnostics: Diagnostics,
    perf_stats: PerfStats,
    detached_windows: HashMap<WindowId, DetachedWindow>,
    detached_tabs: HashMap<DockTab, WindowId>,
    active_drag: Option<DockDragStart>,
    main_cursor_pos: Option<PhysicalPosition<f64>>,
}

struct DetachedWindow {
    dock_state: DockState<DockTab>,
    window: Arc<Window>,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    size: (u32, u32),
    last_pos: Option<PhysicalPosition<i32>>,
    last_cursor_pos: Option<PhysicalPosition<f64>>,
    user_moved: bool,
}

impl App {
    fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let mut renderer = pollster::block_on(Renderer::new(window.as_ref()))?;
        let world = WorldState::seeded();
        let ui = UiState::new();
        let main_window_id = window.id();

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
            0.0,
            0.0,
            0.0,
        );
        let (tile_fetcher, tile_rx) = TileFetcher::new(6);
        let mut tile_layers = TileLayers::new();
        let mut tile_request_id = 0;
        tile_layers.apply_settings(
            &overlay_settings,
            &mut renderer,
            &tile_fetcher,
            &mut tile_request_id,
        );
        Ok(Self {
            window,
            main_window_id,
            renderer,
            world,
            ui,
            instances: Vec::new(),
            filtered_instances: Vec::new(),
            render_instances: Vec::new(),
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
            cull_dirty: true,
            tile_fetcher,
            tile_rx,
            tile_request_id,
            tile_layers,
            diagnostics: Diagnostics::default(),
            perf_stats: PerfStats::new(),
            detached_windows: HashMap::new(),
            detached_tabs: HashMap::new(),
            active_drag: None,
            main_cursor_pos: None,
        })
    }

    fn register_detached_window(
        &mut self,
        dock_state: DockState<DockTab>,
        window: Arc<Window>,
    ) -> anyhow::Result<WindowId> {
        let viewport_id = egui::ViewportId::from_hash_of((window.id(), "detached"));
        let egui_ctx = egui::Context::default();
        egui_ctx.set_style(self.egui_ctx.style().clone());
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            viewport_id,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &self.renderer.device,
            self.renderer.surface_format(),
            egui_wgpu::RendererOptions::default(),
        );
        let (surface, config) = self.renderer.create_window_surface(window.as_ref())?;
        let size = (config.width, config.height);
        let window_id = window.id();
        for (_, tab) in dock_state.iter_all_tabs() {
            self.detached_tabs.insert(*tab, window_id);
        }
        self.detached_windows.insert(
            window_id,
            DetachedWindow {
                dock_state,
                window,
                egui_ctx,
                egui_state,
                egui_renderer,
                surface,
                config,
                size,
                last_pos: None,
                last_cursor_pos: None,
                user_moved: false,
            },
        );
        Ok(window_id)
    }

    fn is_detached_window(&self, window_id: WindowId) -> bool {
        self.detached_windows.contains_key(&window_id)
    }

    fn request_redraws(&self) {
        self.window.request_redraw();
        for window in self.detached_windows.values() {
            window.window.request_redraw();
        }
    }

    fn handle_detached_window_event(&mut self, window_id: WindowId, event: &WindowEvent) {
        let (window, close_requested, moved, resize, user_moved, drag_released) = {
            let Some(detached) = self.detached_windows.get_mut(&window_id) else {
                return;
            };
            let window = Arc::clone(&detached.window);
            let _ = detached.egui_state.on_window_event(window.as_ref(), event);
            let mut close_requested = false;
            let mut moved = false;
            let mut resize = None;
            let mut drag_released = false;
            match event {
                WindowEvent::CloseRequested => {
                    close_requested = true;
                }
                WindowEvent::Resized(size) => {
                    resize = Some((size.width, size.height));
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    let size = window.inner_size();
                    resize = Some((size.width, size.height));
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if *state == winit::event::ElementState::Released
                        && *button == winit::event::MouseButton::Left
                    {
                        drag_released = true;
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    detached.last_cursor_pos = Some(*position);
                }
                WindowEvent::Moved(pos) => {
                    moved = true;
                    let pos = *pos;
                    if let Some(prev) = detached.last_pos {
                        let dx = (pos.x - prev.x).abs();
                        let dy = (pos.y - prev.y).abs();
                        if dx + dy >= 8 {
                            detached.user_moved = true;
                        }
                    }
                    detached.last_pos = Some(pos);
                }
                _ => {}
            }
            (
                window,
                close_requested,
                moved,
                resize,
                detached.user_moved,
                drag_released,
            )
        };

        if let Some((width, height)) = resize {
            self.resize_detached_window(window_id, width, height);
        }
        if drag_released {
            self.handle_drag_release(window_id);
        }
        if close_requested || (moved && user_moved && self.should_attach_to_main(window.as_ref())) {
            self.dock_back_window(window_id);
        }
    }

    fn render_detached_window(&mut self, window_id: WindowId) -> anyhow::Result<()> {
        let Some(detached) = self.detached_windows.get_mut(&window_id) else {
            return Ok(());
        };
        let window = detached.window.as_ref();
        let raw_input = detached.egui_state.take_egui_input(window);
        let output = detached.egui_ctx.run(raw_input, |ctx| {
            self.ui.show_detached_panel(
                ctx,
                DockHost::Detached(window_id.into()),
                &mut detached.dock_state,
                &self.world,
                &self.renderer,
                &self.diagnostics,
            );
        });
        detached
            .egui_state
            .handle_platform_output(window, output.platform_output);
        let paint_jobs = detached
            .egui_ctx
            .tessellate(output.shapes, output.pixels_per_point);

        for (id, image_delta) in &output.textures_delta.set {
            detached.egui_renderer.update_texture(
                &self.renderer.device,
                &self.renderer.queue,
                *id,
                image_delta,
            );
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [detached.size.0, detached.size.1],
            pixels_per_point: output.pixels_per_point,
        };

        let surface_texture = match detached.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
                let width = detached.size.0.max(1);
                let height = detached.size.1.max(1);
                detached.config.width = width;
                detached.config.height = height;
                detached.surface.configure(&self.renderer.device, &detached.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(anyhow::anyhow!("detached surface out of memory"));
            }
            Err(err) => {
                return Err(anyhow::anyhow!("detached surface error: {err:?}"));
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("c2-native detached encoder"),
            });

        let mut egui_cmds = detached.egui_renderer.update_buffers(
            &self.renderer.device,
            &self.renderer.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let egui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui detached pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut egui_pass = egui_pass.forget_lifetime();
            detached.egui_renderer
                .render(&mut egui_pass, &paint_jobs, &screen_descriptor);
        }

        egui_cmds.push(encoder.finish());
        self.renderer.queue.submit(egui_cmds);
        surface_texture.present();

        for id in &output.textures_delta.free {
            detached.egui_renderer.free_texture(id);
        }
        self.capture_drag_start();

        Ok(())
    }

    fn detach_tab(&mut self, target: &ActiveEventLoop, request: DockDetachRequest) -> anyhow::Result<()> {
        if request.tab == DockTab::Globe {
            return Ok(());
        }
        self.remove_tab_from_host(request.host, request.tab);
        let (title, size) = Self::detached_window_spec(request.tab);
        let dock_state = DockState::new(vec![request.tab]);
        let window = Arc::new(target.create_window(
            WindowAttributes::default()
                .with_title(title)
                .with_inner_size(size),
        )?);
        let window_id = self.register_detached_window(dock_state, window)?;
        let window = if let Some(detached) = self.detached_windows.get_mut(&window_id) {
            detached.user_moved = false;
            detached.last_pos = None;
            Arc::clone(&detached.window)
        } else {
            return Ok(());
        };
        self.position_detached_window(request.tab, window.as_ref());
        window.request_redraw();
        Ok(())
    }

    fn dock_back_window(&mut self, window_id: WindowId) {
        let Some(detached) = self.detached_windows.remove(&window_id) else {
            return;
        };
        let tabs: Vec<DockTab> = detached
            .dock_state
            .iter_all_tabs()
            .map(|(_, tab)| *tab)
            .collect();
        for tab in tabs {
            self.detached_tabs.remove(&tab);
            self.ui.main_dock_mut().push_to_focused_leaf(tab);
        }
        detached.window.set_visible(false);
        self.position_hidden_window(detached.window.as_ref());
    }

    fn process_ui_requests(&mut self, target: &ActiveEventLoop) {
        for request in self.ui.take_detach_requests() {
            if let Err(error) = self.detach_tab(target, request) {
                eprintln!("detach failed: {error:?}");
            }
        }
        for host in self.ui.take_attach_requests() {
            if let DockHost::Detached(window_id) = host {
                self.dock_back_window(WindowId::from(window_id));
            }
        }
    }

    fn capture_drag_start(&mut self) {
        let Some(start) = self.ui.take_drag_start() else {
            return;
        };
        if self.active_drag.is_none() {
            self.active_drag = Some(start);
        }
    }

    fn handle_drag_release(&mut self, window_id: WindowId) {
        let Some(active_drag) = self.active_drag.take() else {
            return;
        };
        let target_host = self
            .cursor_global_position(window_id)
            .and_then(|pos| self.dock_host_at_global_position(pos))
            .unwrap_or_else(|| self.dock_host_for_window_id(window_id));
        if target_host == active_drag.host {
            return;
        }
        self.move_tab_between_hosts(active_drag.tab, active_drag.host, target_host);
    }

    fn dock_host_for_window_id(&self, window_id: WindowId) -> DockHost {
        if window_id == self.main_window_id {
            DockHost::Main
        } else {
            DockHost::Detached(window_id.into())
        }
    }

    fn cursor_global_position(
        &self,
        window_id: WindowId,
    ) -> Option<PhysicalPosition<f64>> {
        if window_id == self.main_window_id {
            let cursor = self.main_cursor_pos?;
            let window_pos = self.window.outer_position().ok()?;
            return Some(PhysicalPosition::new(
                window_pos.x as f64 + cursor.x,
                window_pos.y as f64 + cursor.y,
            ));
        }
        let detached = self.detached_windows.get(&window_id)?;
        let cursor = detached.last_cursor_pos?;
        let window_pos = detached.window.outer_position().ok()?;
        Some(PhysicalPosition::new(
            window_pos.x as f64 + cursor.x,
            window_pos.y as f64 + cursor.y,
        ))
    }

    fn dock_host_at_global_position(
        &self,
        position: PhysicalPosition<f64>,
    ) -> Option<DockHost> {
        for (window_id, detached) in &self.detached_windows {
            let Ok(window_pos) = detached.window.outer_position() else {
                continue;
            };
            let window_size = detached.window.outer_size();
            if position.x >= window_pos.x as f64
                && position.y >= window_pos.y as f64
                && position.x <= window_pos.x as f64 + window_size.width as f64
                && position.y <= window_pos.y as f64 + window_size.height as f64
            {
                return Some(DockHost::Detached((*window_id).into()));
            }
        }
        if let Ok(window_pos) = self.window.outer_position() {
            let window_size = self.window.outer_size();
            if position.x >= window_pos.x as f64
                && position.y >= window_pos.y as f64
                && position.x <= window_pos.x as f64 + window_size.width as f64
                && position.y <= window_pos.y as f64 + window_size.height as f64
            {
                return Some(DockHost::Main);
            }
        }
        None
    }

    fn dock_state_for_host_mut(
        &mut self,
        host: DockHost,
    ) -> Option<&mut DockState<DockTab>> {
        match host {
            DockHost::Main => Some(self.ui.main_dock_mut()),
            DockHost::Detached(window_id) => self
                .detached_windows
                .get_mut(&WindowId::from(window_id))
                .map(|detached| &mut detached.dock_state),
        }
    }

    fn dock_host_exists(&self, host: DockHost) -> bool {
        match host {
            DockHost::Main => true,
            DockHost::Detached(window_id) => {
                self.detached_windows.contains_key(&WindowId::from(window_id))
            }
        }
    }

    fn remove_tab_from_host(&mut self, host: DockHost, tab: DockTab) {
        let Some(dock_state) = self.dock_state_for_host_mut(host) else {
            return;
        };
        if let Some((surface, node, tab_index)) = dock_state.find_tab(&tab) {
            dock_state.remove_tab((surface, node, tab_index));
        }
        let should_close = match host {
            DockHost::Detached(_) => dock_state.main_surface().is_empty(),
            DockHost::Main => false,
        };
        let window_id = match host {
            DockHost::Detached(window_id) => Some(WindowId::from(window_id)),
            DockHost::Main => None,
        };
        if should_close {
            if let Some(window_id) = window_id {
                self.close_detached_window(window_id);
            }
        }
    }

    fn move_tab_between_hosts(&mut self, tab: DockTab, from: DockHost, to: DockHost) {
        if tab == DockTab::Globe {
            return;
        }
        if !self.dock_host_exists(to) {
            return;
        }
        self.remove_tab_from_host(from, tab);
        let Some(target_state) = self.dock_state_for_host_mut(to) else {
            return;
        };
        target_state.push_to_focused_leaf(tab);
        match to {
            DockHost::Main => {
                self.detached_tabs.remove(&tab);
            }
            DockHost::Detached(window_id) => {
                self.detached_tabs.insert(tab, WindowId::from(window_id));
            }
        }
    }

    fn close_detached_window(&mut self, window_id: WindowId) {
        let Some(detached) = self.detached_windows.remove(&window_id) else {
            return;
        };
        for (_, tab) in detached.dock_state.iter_all_tabs() {
            self.detached_tabs.remove(tab);
        }
        detached.window.set_visible(false);
        self.position_hidden_window(detached.window.as_ref());
    }

    fn sync_operations_settings(&mut self) {
        let new_settings = self.ui.operations().clone();
        if new_settings != self.overlay_settings {
            self.overlay_settings = new_settings;
            filter_instances(
                &self.instances,
                &self.overlay_settings,
                &mut self.filtered_instances,
            );
            self.instances_dirty = false;
            self.cull_dirty = true;
            self.renderer.update_overlay(
                if self.overlay_settings.show_base { 1.0 } else { 0.0 },
                0.0,
                0.0,
                0.0,
            );
            self.tile_layers.apply_settings(
                &self.overlay_settings,
                &mut self.renderer,
                &self.tile_fetcher,
                &mut self.tile_request_id,
            );
        }
    }

    fn should_attach_to_main(&self, detached_window: &Window) -> bool {
        let Ok(main_pos) = self.window.outer_position() else {
            return false;
        };
        let main_size = self.window.outer_size();
        let Ok(det_pos) = detached_window.outer_position() else {
            return false;
        };
        let det_size = detached_window.outer_size();
        let det_center_x = det_pos.x + det_size.width as i32 / 2;
        let det_center_y = det_pos.y + det_size.height as i32 / 2;
        let within_x = det_center_x >= main_pos.x
            && det_center_x <= main_pos.x + main_size.width as i32;
        let within_y = det_center_y >= main_pos.y
            && det_center_y <= main_pos.y + main_size.height as i32;
        within_x && within_y
    }

    fn detached_window_spec(tab: DockTab) -> (&'static str, PhysicalSize<u32>) {
        match tab {
            DockTab::Operations => ("Operations", PhysicalSize::new(340, 720)),
            DockTab::Entities => ("Entities", PhysicalSize::new(340, 720)),
            DockTab::Inspector => ("Inspector", PhysicalSize::new(520, 360)),
            DockTab::Globe => ("Globe", PhysicalSize::new(520, 360)),
        }
    }

    fn position_detached_window(&self, tab: DockTab, window: &Window) {
        let Ok(main_pos) = self.window.outer_position() else {
            return;
        };
        let main_size = self.window.outer_size();
        let offset_y = match tab {
            DockTab::Operations => 40,
            DockTab::Entities => 120,
            DockTab::Inspector => 200,
            DockTab::Globe => 40,
        };
        let x = main_pos.x + main_size.width as i32 + 24;
        let y = main_pos.y + offset_y;
        window.set_outer_position(PhysicalPosition::new(x, y));
    }

    fn position_hidden_window(&self, window: &Window) {
        let offset = 10000;
        let (x, y) = if let Ok(main_pos) = self.window.outer_position() {
            (main_pos.x - offset, main_pos.y - offset)
        } else {
            (-offset, -offset)
        };
        window.set_outer_position(PhysicalPosition::new(x, y));
    }

    fn resize_detached_window(&mut self, window_id: WindowId, width: u32, height: u32) {
        let Some(detached) = self.detached_windows.get_mut(&window_id) else {
            return;
        };
        let width = width.max(1);
        let height = height.max(1);
        detached.size = (width, height);
        detached.config.width = width;
        detached.config.height = height;
        detached.surface.configure(&self.renderer.device, &detached.config);
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
            WindowEvent::CursorMoved { position, .. } => {
                self.main_cursor_pos = Some(*position);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *state == winit::event::ElementState::Released
                    && *button == winit::event::MouseButton::Left
                {
                    self.handle_drag_release(self.main_window_id);
                }
            }
            _ => {}
        }
        true
    }

    fn update_and_render(&mut self) -> anyhow::Result<()> {
        let window = self.window.as_ref();
        let now = Instant::now();
        let frame_start = now;
        let delta = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;
        let fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };

        let mut world_updated = false;
        let world_start = Instant::now();
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
            filter_instances(
                &self.instances,
                &self.overlay_settings,
                &mut self.filtered_instances,
            );
            self.instances_dirty = false;
            self.cull_dirty = true;
        }
        if self.cull_dirty {
            cull_instances_for_render(
                &self.filtered_instances,
                &self.renderer,
                self.world.globe_radius(),
                &mut self.render_instances,
            );
            self.renderer.update_instances(&self.render_instances);
            self.cull_dirty = false;
        }
        let world_ms = world_start.elapsed().as_secs_f32() * 1000.0;

        let tile_start = Instant::now();
        for result in self
            .tile_rx
            .try_iter()
            .take(MAX_TILE_UPLOADS_PER_FRAME)
        {
            self.tile_layers
                .handle_result(&mut self.renderer, result, now);
        }
        self.tile_layers.update(
            &mut self.renderer,
            &mut self.tile_fetcher,
            &self.overlay_settings,
            now,
            &mut self.tile_request_id,
        );
        let tile_bars = self.tile_layers.progress_bars();
        self.tile_layers.update_diagnostics(&mut self.diagnostics, now);
        let tile_ms = tile_start.elapsed().as_secs_f32() * 1000.0;

        let ui_start = Instant::now();
        let raw_input = self.egui_state.take_egui_input(window);

        let output = self.egui_ctx.run(raw_input, |ctx| {
            self.ui.show(
                ctx,
                &self.world,
                &self.renderer,
                &self.filtered_instances,
                self.globe_texture_id,
                &tile_bars,
                &self.diagnostics,
            );
        });
        self.egui_state
            .handle_platform_output(window, output.platform_output);
        self.capture_drag_start();
        let paint_jobs = self.egui_ctx.tessellate(output.shapes, output.pixels_per_point);
        let ui_ms = ui_start.elapsed().as_secs_f32() * 1000.0;

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
                            self.cull_dirty = true;
                        }
                    }
                    if hovered {
                        let scroll = input.smooth_scroll_delta.y;
                        if scroll.abs() > 0.0 {
                            self.renderer.zoom_delta(scroll);
                            self.cull_dirty = true;
                        }
                    }
                } else if input.pointer.primary_released() {
                    self.globe_dragging = false;
                }
            });
        }

        self.sync_operations_settings();

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

        let render_start = Instant::now();
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
        let render_ms = render_start.elapsed().as_secs_f32() * 1000.0;

        for id in &output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let frame_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        self.diagnostics.perf = self.perf_stats.update(
            frame_ms,
            world_ms,
            tile_ms,
            ui_ms,
            render_ms,
            fps,
        );

        Ok(())
    }
}

struct PerfStats {
    samples: Vec<f32>,
    scratch: Vec<f32>,
    index: usize,
    count: usize,
    snapshot: PerfSnapshot,
}

impl PerfStats {
    fn new() -> Self {
        Self {
            samples: vec![0.0; PERF_SAMPLE_COUNT],
            scratch: Vec::with_capacity(PERF_SAMPLE_COUNT),
            index: 0,
            count: 0,
            snapshot: PerfSnapshot::default(),
        }
    }

    fn update(
        &mut self,
        frame_ms: f32,
        world_ms: f32,
        tile_ms: f32,
        ui_ms: f32,
        render_ms: f32,
        fps: f32,
    ) -> PerfSnapshot {
        if !frame_ms.is_finite() {
            return self.snapshot;
        }
        if !self.samples.is_empty() {
            self.samples[self.index] = frame_ms;
            self.index = (self.index + 1) % self.samples.len();
            self.count = (self.count + 1).min(self.samples.len());
        }

        self.scratch.clear();
        if self.count > 0 {
            self.scratch.extend_from_slice(&self.samples[..self.count]);
            self.scratch
                .sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        }
        let p95 = percentile(&self.scratch, 0.95);
        let p99 = percentile(&self.scratch, 0.99);

        self.snapshot = PerfSnapshot {
            fps,
            frame_ms,
            frame_p95_ms: p95,
            frame_p99_ms: p99,
            world_ms,
            tile_ms,
            ui_ms,
            render_ms,
        };
        self.snapshot
    }
}

fn percentile(sorted: &[f32], pct: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f32 * pct.clamp(0.0, 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[derive(Clone)]
struct TileLayers {
    map: TileLayerState,
    weather: TileLayerState,
    sea: TileLayerState,
}

impl TileLayers {
    fn new() -> Self {
        Self {
            map: TileLayerState::new(
                TileKind::Base,
                MAP_TILE_CAPACITY,
                220,
                MAP_TILE_CAPACITY,
                MAP_UPDATE_INTERVAL_MS,
                0.85,
            ),
            weather: TileLayerState::new(
                TileKind::Weather,
                WEATHER_TILE_CAPACITY,
                60,
                WEATHER_TILE_CAPACITY,
                WEATHER_UPDATE_INTERVAL_MS,
                0.55,
            ),
            sea: TileLayerState::new(
                TileKind::Sea,
                SEA_TILE_CAPACITY,
                50,
                SEA_TILE_CAPACITY,
                SEA_UPDATE_INTERVAL_MS,
                0.45,
            ),
        }
    }

    fn apply_settings(
        &mut self,
        settings: &OperationsState,
        renderer: &mut Renderer,
        fetcher: &TileFetcher,
        request_id: &mut u64,
    ) {
        let mut map_dirty = self.map.set_enabled(settings.show_map, renderer);
        let mut weather_dirty = self.weather.set_enabled(settings.show_weather, renderer);
        let mut sea_dirty = self.sea.set_enabled(settings.show_sea, renderer);

        renderer.update_tile_opacity(
            TileKind::Base,
            if settings.show_map { self.map.opacity } else { 0.0 },
        );
        renderer.update_tile_opacity(
            TileKind::Weather,
            if settings.show_weather {
                self.weather.opacity
            } else {
                0.0
            },
        );
        renderer.update_tile_opacity(
            TileKind::Sea,
            if settings.show_sea { self.sea.opacity } else { 0.0 },
        );

        let map_provider = settings.tile_provider.clone();
        if self.map.provider != map_provider {
            self.map.provider = map_provider;
            self.map.reset();
            map_dirty = true;
        }
        let weather_field = settings.weather_field.clone();
        if self.weather.field != weather_field {
            self.weather.field = weather_field;
            self.weather.reset();
            weather_dirty = true;
        }
        let sea_field = settings.sea_field.clone();
        if self.sea.field != sea_field {
            self.sea.field = sea_field;
            self.sea.reset();
            sea_dirty = true;
        }

        if map_dirty {
            self.map.bump_request_id(fetcher, request_id);
        }
        if weather_dirty {
            self.weather.bump_request_id(fetcher, request_id);
        }
        if sea_dirty {
            self.sea.bump_request_id(fetcher, request_id);
        }
    }

    fn update(
        &mut self,
        renderer: &mut Renderer,
        fetcher: &mut TileFetcher,
        settings: &OperationsState,
        now: Instant,
        request_id: &mut u64,
    ) {
        let provider = tile_provider_config(&settings.tile_provider);
        self.map.update(
            renderer,
            fetcher,
            now,
            request_id,
            provider,
            settings,
        );
        self.weather.update(renderer, fetcher, now, request_id, provider, settings);
        self.sea.update(renderer, fetcher, now, request_id, provider, settings);
    }

    fn handle_result(&mut self, renderer: &mut Renderer, result: TileResult, now: Instant) {
        match result.kind {
            TileKind::Base => self.map.handle_result(renderer, result, now),
            TileKind::Weather => self.weather.handle_result(renderer, result, now),
            TileKind::Sea => self.sea.handle_result(renderer, result, now),
        }
    }

    fn progress_bars(&self) -> Vec<crate::ui::TileBar> {
        vec![
            self.map.progress_bar(egui::Color32::from_rgb(86, 156, 255)),
            self.sea.progress_bar(egui::Color32::from_rgb(64, 196, 196)),
            self.weather
                .progress_bar(egui::Color32::from_rgb(255, 164, 72)),
        ]
    }

    fn update_diagnostics(&self, diagnostics: &mut Diagnostics, now: Instant) {
        diagnostics.map = self.map.stats(now);
        diagnostics.weather = self.weather.stats(now);
        diagnostics.sea = self.sea.stats(now);
    }
}

#[derive(Clone)]
struct TileLayerState {
    kind: TileKind,
    enabled: bool,
    provider: String,
    field: String,
    zoom: u8,
    request_id: u64,
    max_tiles: usize,
    max_cache: usize,
    update_interval: std::time::Duration,
    last_update: Instant,
    last_activity: Instant,
    last_direction: Vec3,
    last_distance: f32,
    tiles: std::collections::HashMap<TileKey, TileEntry>,
    pending: std::collections::HashMap<TileKey, u32>,
    desired: Vec<TileKey>,
    atlas: TileAtlas,
    progress_total: usize,
    progress_loaded: usize,
    force_update: bool,
    opacity: f32,
}

impl TileLayerState {
    fn new(
        kind: TileKind,
        capacity: usize,
        max_tiles: usize,
        max_cache: usize,
        update_interval_ms: u64,
        opacity: f32,
    ) -> Self {
        Self {
            kind,
            enabled: true,
            provider: String::new(),
            field: String::new(),
            zoom: 0,
            request_id: 0,
            max_tiles,
            max_cache,
            update_interval: std::time::Duration::from_millis(update_interval_ms),
            last_update: Instant::now(),
            last_activity: Instant::now(),
            last_direction: Vec3::ZERO,
            last_distance: 0.0,
            tiles: std::collections::HashMap::new(),
            pending: std::collections::HashMap::new(),
            desired: Vec::new(),
            atlas: TileAtlas::new(capacity as u32),
            progress_total: 0,
            progress_loaded: 0,
            force_update: true,
            opacity,
        }
    }

    fn set_enabled(&mut self, enabled: bool, renderer: &mut Renderer) -> bool {
        if self.enabled == enabled {
            return false;
        }
        self.enabled = enabled;
        self.last_activity = Instant::now();
        if !enabled {
            self.pending.clear();
            self.desired.clear();
            self.progress_total = 0;
            self.progress_loaded = 0;
            renderer.update_tile_instances(self.kind, &[]);
            renderer.update_tile_opacity(self.kind, 0.0);
        } else {
            renderer.update_tile_opacity(self.kind, self.opacity);
            self.force_update = true;
        }
        true
    }

    fn reset(&mut self) {
        self.tiles.clear();
        self.pending.clear();
        self.desired.clear();
        self.atlas.reset();
        self.progress_total = 0;
        self.progress_loaded = 0;
        self.force_update = true;
        self.zoom = 0;
        self.last_activity = Instant::now();
    }

    fn update(
        &mut self,
        renderer: &mut Renderer,
        fetcher: &mut TileFetcher,
        now: Instant,
        request_id: &mut u64,
        provider: TileProviderConfig,
        settings: &OperationsState,
    ) {
        if !self.enabled {
            return;
        }

        let desired_zoom = match self.kind {
            TileKind::Base => pick_tile_zoom(renderer, provider, DEFAULT_GLOBE_RADIUS),
            TileKind::Weather => {
                pick_overlay_zoom(renderer, WEATHER_MIN_ZOOM, WEATHER_MAX_ZOOM, DEFAULT_GLOBE_RADIUS)
            }
            TileKind::Sea => {
                pick_overlay_zoom(renderer, SEA_MIN_ZOOM, SEA_MAX_ZOOM, DEFAULT_GLOBE_RADIUS)
            }
        };
        let mut needs_new_request = self.request_id == 0;
        if desired_zoom != self.zoom {
            self.zoom = desired_zoom;
            self.tiles.clear();
            self.pending.clear();
            self.desired.clear();
            self.atlas.reset();
            self.progress_total = 0;
            self.progress_loaded = 0;
            self.force_update = true;
            needs_new_request = true;
        }
        if needs_new_request {
            self.bump_request_id(fetcher, request_id);
        }

        let should_update = self.should_update(renderer, now);
        if !should_update && !self.force_update {
            self.progress_loaded = self
                .desired
                .iter()
                .filter(|key| self.tiles.contains_key(key))
                .count();
            self.progress_total = self.desired.len();
            renderer.update_tile_instances(self.kind, &self.build_instances());
            return;
        }

        self.force_update = false;
        self.last_update = now;
        let camera_dir = renderer.camera_position().normalize_or_zero();
        self.last_direction = camera_dir;
        self.last_distance = renderer.camera_distance();

        let desired = compute_visible_tiles(renderer, self.zoom, self.max_tiles);
        self.desired = desired.clone();
        let desired_set: std::collections::HashSet<TileKey> = desired.iter().copied().collect();

        for (key, entry) in self.tiles.iter_mut() {
            entry.visible = desired_set.contains(key);
            if entry.visible {
                entry.last_used = now;
            }
        }
        if !self.pending.is_empty() {
            let mut stale = Vec::new();
            for (key, index) in self.pending.iter() {
                if !desired_set.contains(key) {
                    stale.push((*key, *index));
                }
            }
            if !stale.is_empty() {
                for (key, index) in stale {
                    self.pending.remove(&key);
                    self.atlas.free(index);
                }
            }
        }

        self.progress_total = desired.len();
        self.progress_loaded = 0;
        let mut instances = Vec::new();
        let mut sent_any = false;

        for key in desired.iter() {
            if let Some(entry) = self.tiles.get_mut(key) {
                self.progress_loaded += 1;
                entry.last_used = now;
                let bounds = tile_bounds(*key);
                instances.push(TileInstanceRaw {
                    bounds: [bounds.lon_min, bounds.lon_max, bounds.merc_min, bounds.merc_max],
                    layer: entry.layer_index as f32,
                });
                continue;
            }
            if self.pending.contains_key(key) {
                continue;
            }
            if !self.atlas.has_free() || self.tiles.len() + self.pending.len() >= self.max_cache {
                self.evict_cache(now);
            }
            let Some(layer_index) = self.atlas.alloc() else {
                continue;
            };
            let request = TileRequest {
                request_id: self.request_id,
                kind: self.kind,
                key: *key,
                provider: settings.tile_provider.clone(),
                weather_field: settings.weather_field.clone(),
                sea_field: settings.sea_field.clone(),
                layer_index,
            };
            if fetcher.request(request) {
                self.pending.insert(*key, layer_index);
                sent_any = true;
            } else {
                self.atlas.free(layer_index);
            }
        }

        if sent_any {
            self.last_activity = now;
        }

        renderer.update_tile_instances(self.kind, &instances);
    }

    fn handle_result(&mut self, renderer: &mut Renderer, result: TileResult, now: Instant) {
        if result.request_id != self.request_id {
            return;
        }
        self.last_activity = now;
        let layer_index = match self.pending.remove(&result.key) {
            Some(index) => index,
            None => result.layer_index,
        };
        if !result.valid {
            self.atlas.free(layer_index);
            return;
        }
        renderer.update_tile_texture(
            self.kind,
            layer_index,
            result.width,
            result.height,
            &result.data,
        );
        self.tiles.insert(
            result.key,
            TileEntry {
                layer_index,
                last_used: now,
                visible: true,
            },
        );
    }

    fn progress_bar(&self, color: egui::Color32) -> crate::ui::TileBar {
        let pending = self.pending.len();
        let loaded = self.progress_loaded;
        let total = loaded + pending;
        let has_work = self.enabled && pending > 0 && total > 0;
        let progress = if has_work {
            Some(loaded as f32 / total as f32)
        } else {
            None
        };
        crate::ui::TileBar {
            enabled: has_work,
            progress,
            color,
        }
    }

    fn stats(&self, now: Instant) -> crate::ui::TileLayerStats {
        let last_activity_ms = if self.enabled {
            now.duration_since(self.last_activity).as_secs_f32() * 1000.0
        } else {
            0.0
        };
        let stalled = self.enabled
            && self.pending.len() > 0
            && last_activity_ms > TILE_STALL_THRESHOLD_MS;
        crate::ui::TileLayerStats {
            enabled: self.enabled,
            zoom: self.zoom,
            desired: self.desired.len(),
            loaded: self.progress_loaded,
            pending: self.pending.len(),
            cache_used: self.tiles.len() + self.pending.len(),
            cache_cap: self.max_cache,
            last_activity_ms,
            stalled,
        }
    }

    fn build_instances(&self) -> Vec<TileInstanceRaw> {
        let mut instances = Vec::new();
        for key in self.desired.iter() {
            if let Some(entry) = self.tiles.get(key) {
                let bounds = tile_bounds(*key);
                instances.push(TileInstanceRaw {
                    bounds: [bounds.lon_min, bounds.lon_max, bounds.merc_min, bounds.merc_max],
                    layer: entry.layer_index as f32,
                });
            }
        }
        instances
    }

    fn should_update(&self, renderer: &Renderer, now: Instant) -> bool {
        if now.duration_since(self.last_update) < self.update_interval {
            let dir = renderer.camera_position().normalize_or_zero();
            let distance = renderer.camera_distance();
            let moved = dir.dot(self.last_direction) < 0.999
                || (distance - self.last_distance).abs() > (distance * 0.0015).max(0.08);
            if !moved {
                return false;
            }
        }
        true
    }

    fn evict_cache(&mut self, _now: Instant) {
        if self.tiles.len() <= self.max_cache {
            return;
        }
        let mut entries: Vec<(TileKey, bool, Instant)> = self
            .tiles
            .iter()
            .map(|(key, entry)| (*key, entry.visible, entry.last_used))
            .collect();
        entries.sort_by(|a, b| {
            match (a.1, b.1) {
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                _ => a.2.cmp(&b.2),
            }
        });
        for (key, _visible, _) in entries {
            if self.tiles.len() <= self.max_cache {
                break;
            }
            if let Some(entry) = self.tiles.remove(&key) {
                self.atlas.free(entry.layer_index);
                self.pending.remove(&key);
            }
        }
    }

    fn bump_request_id(&mut self, fetcher: &TileFetcher, request_id: &mut u64) {
        *request_id = request_id.wrapping_add(1);
        self.request_id = *request_id;
        fetcher.set_current_request_id(self.kind, self.request_id);
    }
}

#[derive(Clone, Copy)]
struct TileEntry {
    layer_index: u32,
    last_used: Instant,
    visible: bool,
}

#[derive(Clone)]
struct TileAtlas {
    capacity: u32,
    free: Vec<u32>,
}

impl TileAtlas {
    fn new(capacity: u32) -> Self {
        let mut free = Vec::with_capacity(capacity as usize);
        for i in (0..capacity).rev() {
            free.push(i);
        }
        Self { capacity, free }
    }

    fn reset(&mut self) {
        self.free.clear();
        for i in (0..self.capacity).rev() {
            self.free.push(i);
        }
    }

    fn alloc(&mut self) -> Option<u32> {
        self.free.pop()
    }

    fn has_free(&self) -> bool {
        !self.free.is_empty()
    }

    fn free(&mut self, index: u32) {
        if index < self.capacity {
            self.free.push(index);
        }
    }
}

#[derive(Clone, Copy)]
struct GeoSample {
    lat: f32,
    lon: f32,
}

#[derive(Clone, Copy)]
struct TileBounds {
    lon_min: f32,
    lon_max: f32,
    merc_min: f32,
    merc_max: f32,
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

fn cull_instances_for_render(
    instances: &[RenderInstance],
    renderer: &Renderer,
    globe_radius: f32,
    out: &mut Vec<RenderInstance>,
) {
    let camera_pos = renderer.camera_position();
    out.clear();
    for instance in instances {
        if is_occluded_by_globe(camera_pos, instance.position, globe_radius) {
            continue;
        }
        out.push(*instance);
    }
}

fn compute_visible_tiles(renderer: &Renderer, zoom: u8, max_tiles: usize) -> Vec<TileKey> {
    let (width, height) = renderer.viewport_size();
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let center = match sample_geo(renderer, 0.0, 0.0, DEFAULT_GLOBE_RADIUS) {
        Some(center) => center,
        None => return Vec::new(),
    };
    let focus_px = pick_focus_box_px(width, height, zoom);
    let ndc_x = (focus_px / width as f32 * 2.0).min(0.95);
    let ndc_y = (focus_px / height as f32 * 2.0).min(0.95);
    let samples = [
        (0.0, 0.0),
        (ndc_x, 0.0),
        (-ndc_x, 0.0),
        (0.0, ndc_y),
        (0.0, -ndc_y),
        (ndc_x, ndc_y),
        (-ndc_x, ndc_y),
        (ndc_x, -ndc_y),
        (-ndc_x, -ndc_y),
    ];
    let mut geos = Vec::new();
    for (x, y) in samples.iter() {
        if let Some(sample) = sample_geo(renderer, *x, *y, DEFAULT_GLOBE_RADIUS) {
            geos.push(sample);
        }
    }
    if geos.is_empty() {
        return Vec::new();
    }

    let tile_lons: Vec<f32> = geos.iter().map(|geo| flip_lon(geo.lon)).collect();
    let lat_min = geos
        .iter()
        .map(|geo| geo.lat)
        .fold(f32::INFINITY, f32::min)
        .max(-85.0);
    let lat_max = geos
        .iter()
        .map(|geo| geo.lat)
        .fold(f32::NEG_INFINITY, f32::max)
        .min(85.0);
    let lon_range = compute_lon_range(&tile_lons);
    let (lon_min, lon_max) = match lon_range {
        Some(range) => range,
        None => (-180.0, 180.0),
    };
    let lon_span = lon_max - lon_min;
    let lon_padding = (lon_span * 0.04).max(1.0);
    let lon_min = lon_min - lon_padding;
    let lon_max = lon_max + lon_padding;

    let n = 1u32 << zoom;
    let center_lon = flip_lon(center.lon);
    let center_x = tile_x_for_lon(center_lon, zoom);
    let center_y = tile_y_for_lat(center.lat, zoom);
    let y_min = tile_y_for_lat(lat_max, zoom).saturating_sub(1);
    let y_max = tile_y_for_lat(lat_min, zoom).saturating_add(1).min(n - 1);

    let mut ranges = Vec::new();
    if lon_span >= 360.0 {
        ranges.push((-180.0, 180.0));
    } else if lon_min < -180.0 {
        ranges.push((lon_min + 360.0, 180.0));
        ranges.push((-180.0, lon_max));
    } else if lon_max > 180.0 {
        ranges.push((lon_min, 180.0));
        ranges.push((-180.0, lon_max - 360.0));
    } else {
        ranges.push((lon_min, lon_max));
    }

    let mut candidates = Vec::new();
    for (range_min, range_max) in ranges {
        let start_x = tile_x_for_lon(range_min, zoom);
        let end_x = tile_x_for_lon(range_max, zoom);
        for x in start_x.saturating_sub(1)..=end_x.saturating_add(1) {
            let wrapped_x = ((x as i64 % n as i64) + n as i64) as u32 % n;
            for y in y_min..=y_max {
                let key = TileKey {
                    zoom,
                    x: wrapped_x,
                    y,
                };
                let dx = wrap_tile_delta(x as i64 - center_x as i64, n as i64) as f32;
                let dy = y as f32 - center_y as f32;
                let dist = dx * dx + dy * dy;
                candidates.push((key, dist));
            }
        }
    }
    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    candidates
        .into_iter()
        .take(max_tiles)
        .map(|(key, _)| key)
        .collect()
}

fn wrap_tile_delta(delta: i64, tiles: i64) -> i64 {
    let mut value = delta % tiles;
    if value > tiles / 2 {
        value -= tiles;
    } else if value < -tiles / 2 {
        value += tiles;
    }
    value
}

fn wrap_lon(mut lon: f32) -> f32 {
    while lon > 180.0 {
        lon -= 360.0;
    }
    while lon < -180.0 {
        lon += 360.0;
    }
    lon
}

fn flip_lon(lon: f32) -> f32 {
    wrap_lon(180.0 - lon)
}

fn pick_focus_box_px(width: u32, height: u32, zoom: u8) -> f32 {
    let base = (width.min(height) as f32).max(1.0);
    let max_radius = (base * 0.32).max(220.0);
    let min_radius = (base * 0.2).max(140.0);
    let ratio = (zoom as f32 / 6.0).clamp(0.0, 1.0);
    min_radius + (max_radius - min_radius) * ratio
}

fn sample_geo(
    renderer: &Renderer,
    ndc_x: f32,
    ndc_y: f32,
    radius: f32,
) -> Option<GeoSample> {
    let inv = renderer.view_proj().inverse();
    let near = inv.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
    let far = inv.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));
    let origin = renderer.camera_position();
    let dir = (far - near).normalize_or_zero();
    let t = ray_sphere_intersect(origin, dir, radius)?;
    let hit = origin + dir * t;
    let lat = (hit.y / radius).clamp(-1.0, 1.0).asin().to_degrees();
    let lon = hit.z.atan2(hit.x).to_degrees();
    Some(GeoSample { lat, lon })
}

fn ray_sphere_intersect(origin: Vec3, dir: Vec3, radius: f32) -> Option<f32> {
    let b = origin.dot(dir);
    let c = origin.length_squared() - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let sqrt_disc = disc.sqrt();
    let mut t = -b - sqrt_disc;
    if t <= 0.0 {
        t = -b + sqrt_disc;
    }
    if t <= 0.0 {
        None
    } else {
        Some(t)
    }
}

fn is_occluded_by_globe(camera_pos: Vec3, target: Vec3, radius: f32) -> bool {
    let delta = target - camera_pos;
    let dist = delta.length();
    if dist <= f32::EPSILON {
        return false;
    }
    let dir = delta / dist;
    if let Some(t) = ray_sphere_intersect(camera_pos, dir, radius) {
        t < dist
    } else {
        false
    }
}

fn tile_x_for_lon(lon: f32, zoom: u8) -> u32 {
    let n = 1u32 << zoom;
    let mut value = ((lon + 180.0) / 360.0 * n as f32).floor() as i64;
    if value < 0 {
        value += n as i64;
    }
    (value as u32).min(n - 1)
}

fn tile_y_for_lat(lat: f32, zoom: u8) -> u32 {
    let n = 1u32 << zoom;
    let lat = lat.clamp(-85.0511, 85.0511).to_radians();
    let y = (1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / std::f32::consts::PI) / 2.0;
    (y * n as f32).floor().clamp(0.0, (n - 1) as f32) as u32
}

fn tile_bounds(key: TileKey) -> TileBounds {
    let n = 1u32 << key.zoom;
    let lon_min = key.x as f32 / n as f32 * 360.0 - 180.0;
    let lon_max = (key.x + 1) as f32 / n as f32 * 360.0 - 180.0;
    let merc_min = key.y as f32 / n as f32;
    let merc_max = (key.y + 1) as f32 / n as f32;
    TileBounds {
        lon_min,
        lon_max,
        merc_min,
        merc_max,
    }
}

fn tile_lat_from_y(y: u32, zoom: u8) -> f32 {
    let n = 1u32 << zoom;
    let y = y as f32 / n as f32;
    let lat = (std::f32::consts::PI * (1.0 - 2.0 * y)).sinh().atan();
    lat.to_degrees()
}

fn compute_lon_range(lons: &[f32]) -> Option<(f32, f32)> {
    if lons.is_empty() {
        return None;
    }
    let mut sum_sin = 0.0;
    let mut sum_cos = 0.0;
    for lon in lons {
        let rad = lon.to_radians();
        sum_sin += rad.sin();
        sum_cos += rad.cos();
    }
    let mean = sum_sin.atan2(sum_cos).to_degrees();
    let mut min_delta: f32 = 180.0;
    let mut max_delta: f32 = -180.0;
    for lon in lons {
        let mut delta = lon - mean;
        delta = ((delta + 540.0) % 360.0) - 180.0;
        min_delta = min_delta.min(delta);
        max_delta = max_delta.max(delta);
    }
    Some((mean + min_delta, mean + max_delta))
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
    let tile_deg_width = deg_width * (TILE_SIZE as f32 / width as f32);
    let tile_deg_height = deg_height * (TILE_SIZE as f32 / height as f32);
    let tile_deg = tile_deg_width.max(tile_deg_height).max(0.0001);
    let mut zoom = (360.0 / tile_deg).log2().round() as i32 + zoom_bias as i32;
    let max_zoom = max_zoom.min(TILE_ZOOM_CAP) as i32;
    let min_zoom = min_zoom as i32;
    zoom = zoom.clamp(min_zoom, max_zoom);
    zoom as u8
}

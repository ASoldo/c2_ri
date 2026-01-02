use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use glam::Vec3;
use iced::mouse::Cursor;
use iced::{Point, Rectangle, Size, Theme};
use iced_wgpu::graphics::{Shell, Viewport};
use iced_wgpu::Engine;
use iced_winit::conversion;
use iced_winit::core::{renderer, Event as IcedEvent};
use iced_winit::runtime::user_interface::{Cache, UserInterface};
use iced_winit::Clipboard;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{
    DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, StartCause, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId};

use crate::ecs::{RenderInstance, WorldState, KIND_FLIGHT, KIND_SATELLITE, KIND_SHIP};
use crate::renderer::{RenderBounds, Renderer, TileInstanceRaw};
use crate::tiles::{
    TileFetcher, TileKey, TileKind, TileRequest, TileResult, MAP_TILE_CAPACITY, SEA_TILE_CAPACITY,
    WEATHER_TILE_CAPACITY,
};
use crate::ui::{
    tile_provider_config, Diagnostics, DragPreview, DropIndicator, MainPanels, OperationsState,
    PanelId, PerfSnapshot, TileBar, TileProviderConfig, UiMessage, UiState,
};

const DEFAULT_GLOBE_RADIUS: f32 = 120.0;
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
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct App {
    core: Option<AppCore>,
    main: Option<MainWindow>,
    detached: HashMap<WindowId, DetachedWindow>,
    modifiers: ModifiersState,
    drag_state: Option<DragState>,
    global_cursor: Option<PhysicalPosition<f64>>,
    hovered_window: Option<WindowId>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.core.is_some() {
            return;
        }

        let window = match event_loop
            .create_window(WindowAttributes::default().with_title("C2 Walaris"))
        {
            Ok(window) => Arc::new(window),
            Err(err) => {
                eprintln!("failed to create main window: {err:?}");
                event_loop.exit();
                return;
            }
        };

        let renderer = match pollster::block_on(Renderer::new(window.as_ref())) {
            Ok(renderer) => renderer,
            Err(err) => {
                eprintln!("failed to create renderer: {err:?}");
                event_loop.exit();
                return;
            }
        };

        let ui_renderer = build_ui_renderer(&renderer, renderer.surface_format());

        let core = match AppCore::new(renderer) {
            Ok(core) => core,
            Err(err) => {
                eprintln!("failed to initialize core: {err:?}");
                event_loop.exit();
                return;
            }
        };

        let main = MainWindow::new(window, ui_renderer);

        self.core = Some(core);
        self.main = Some(main);
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if self
            .main
            .as_ref()
            .is_some_and(|main| main.window.id() == window_id)
        {
            self.handle_main_window_event(event_loop, event);
            return;
        }

        if self.detached.contains_key(&window_id) {
            self.handle_detached_window_event(event_loop, window_id, event);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                let mut update_drop_targets = false;
                if let Some(cursor) = self.global_cursor.as_mut() {
                    cursor.x += delta.0;
                    cursor.y += delta.1;
                    update_drop_targets = true;
                }
                if update_drop_targets {
                    self.update_drop_targets();
                }
            }
            DeviceEvent::Button { button, state } => {
                if button == 0 && state == ElementState::Released {
                    if let Some(main) = self.main.as_mut() {
                        main.globe_dragging = false;
                        main.last_cursor_physical = None;
                    }
                    for detached in self.detached.values_mut() {
                        detached.globe_dragging = false;
                        detached.last_cursor_physical = None;
                    }
                    self.finish_drag(event_loop);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(main) = self.main.as_ref() {
            main.window.request_redraw();
        }
        for window in self.detached.values() {
            window.window.request_redraw();
        }
    }
}

impl App {
    fn handle_main_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        if matches!(&event, WindowEvent::CloseRequested) {
            event_loop.exit();
            return;
        }
        if matches!(&event, WindowEvent::RedrawRequested) {
            if let Err(err) = self.render_main(event_loop) {
                eprintln!("render error: {err:?}");
            }
            return;
        }

        let scale_factor = self
            .main
            .as_ref()
            .map(|main| main.window.scale_factor() as f32)
            .unwrap_or(1.0);
        let mut update_drop_targets = false;
        let mut finish_drag = false;
        let mut update_global = None;

        {
            let Some(core) = self.core.as_mut() else {
                return;
            };
            let Some(main) = self.main.as_mut() else {
                return;
            };

            match &event {
                WindowEvent::CursorEntered { .. } => {
                    self.hovered_window = Some(main.window.id());
                    update_drop_targets = true;
                }
                WindowEvent::Resized(size) => {
                    core.renderer.resize(size.width, size.height);
                    core.cull_dirty = true;
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    let size = main.window.inner_size();
                    core.renderer.resize(size.width, size.height);
                    core.cull_dirty = true;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let logical = position.to_logical::<f64>(f64::from(scale_factor));
                    main.cursor_logical = Some(Point::new(logical.x as f32, logical.y as f32));
                    update_global = Some((main.window.clone(), *position));
                    if let Some(last_pos) = main.last_cursor_physical {
                        if main.globe_dragging {
                            let dx = position.x - last_pos.x;
                            let dy = position.y - last_pos.y;
                            if dx != 0.0 || dy != 0.0 {
                                core.renderer.orbit_delta(dx as f32, dy as f32);
                                core.cull_dirty = true;
                            }
                        }
                    }
                    main.last_cursor_physical = Some(*position);
                    update_drop_targets = true;
                }
                WindowEvent::CursorLeft { .. } => {
                    main.cursor_logical = None;
                    main.last_cursor_physical = None;
                    if self.drag_state.is_none() {
                        self.global_cursor = None;
                    }
                    if self.hovered_window == Some(main.window.id()) {
                        self.hovered_window = None;
                    }
                    update_drop_targets = true;
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if *button == MouseButton::Left {
                        match state {
                            ElementState::Pressed => {
                                if cursor_in_globe(core, main) {
                                    main.globe_dragging = true;
                                }
                            }
                            ElementState::Released => {
                                main.globe_dragging = false;
                                finish_drag = true;
                            }
                        }
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if cursor_in_globe(core, main) {
                        let scroll = match delta {
                            MouseScrollDelta::LineDelta(_, y) => *y,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        };
                        core.renderer.zoom_delta(scroll);
                        core.cull_dirty = true;
                    }
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.modifiers = modifiers.state();
                }
                _ => {}
            }

            if let Some(iced_event) = conversion::window_event(event, scale_factor, self.modifiers) {
                main.ui_events.push(iced_event);
            }
        }

        if let Some((window, position)) = update_global {
            self.update_global_cursor(&window, position);
        }
        if update_drop_targets {
            self.update_drop_targets();
        }
        if finish_drag {
            self.finish_drag(event_loop);
        }
    }

    fn handle_detached_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(&event, WindowEvent::CloseRequested) {
            self.dock_back(window_id);
            return;
        }
        if matches!(&event, WindowEvent::RedrawRequested) {
            if let Err(err) = self.render_detached(event_loop, window_id) {
                eprintln!("render error: {err:?}");
            }
            return;
        }

        let scale_factor = self
            .detached
            .get(&window_id)
            .map(|detached| detached.window.scale_factor() as f32)
            .unwrap_or(1.0);
        let mut update_drop_targets = false;
        let mut finish_drag = false;
        let mut update_global = None;

        {
            let Some(core) = self.core.as_mut() else {
                return;
            };
            let Some(detached) = self.detached.get_mut(&window_id) else {
                return;
            };
            let globe_active = detached.active_panel == PanelId::Globe;

            match &event {
                WindowEvent::CursorEntered { .. } => {
                    self.hovered_window = Some(detached.window.id());
                    update_drop_targets = true;
                }
                WindowEvent::Resized(size) => {
                    resize_detached_window(core, detached, *size);
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    let size = detached.window.inner_size();
                    resize_detached_window(core, detached, size);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let logical = position.to_logical::<f64>(f64::from(scale_factor));
                    detached.cursor_logical = Some(Point::new(logical.x as f32, logical.y as f32));
                    update_global = Some((detached.window.clone(), *position));
                    if let Some(last_pos) = detached.last_cursor_physical {
                        if globe_active && detached.globe_dragging {
                            let dx = position.x - last_pos.x;
                            let dy = position.y - last_pos.y;
                            if dx != 0.0 || dy != 0.0 {
                                core.renderer.orbit_delta(dx as f32, dy as f32);
                                core.cull_dirty = true;
                            }
                        }
                    }
                    detached.last_cursor_physical = Some(*position);
                    update_drop_targets = true;
                }
                WindowEvent::CursorLeft { .. } => {
                    detached.cursor_logical = None;
                    detached.last_cursor_physical = None;
                    if self.drag_state.is_none() {
                        self.global_cursor = None;
                    }
                    if self.hovered_window == Some(detached.window.id()) {
                        self.hovered_window = None;
                    }
                    update_drop_targets = true;
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if *button == MouseButton::Left {
                        match state {
                            ElementState::Pressed => {
                                if globe_active && cursor_in_detached_globe(core, detached) {
                                    detached.globe_dragging = true;
                                }
                            }
                            ElementState::Released => {
                                detached.globe_dragging = false;
                                finish_drag = true;
                            }
                        }
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if globe_active && cursor_in_detached_globe(core, detached) {
                        let scroll = match delta {
                            MouseScrollDelta::LineDelta(_, y) => *y,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        };
                        core.renderer.zoom_delta(scroll);
                        core.cull_dirty = true;
                    }
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.modifiers = modifiers.state();
                }
                _ => {}
            }

            if let Some(iced_event) = conversion::window_event(event, scale_factor, self.modifiers) {
                detached.ui_events.push(iced_event);
            }
        }

        if let Some((window, position)) = update_global {
            self.update_global_cursor(&window, position);
        }
        if update_drop_targets {
            self.update_drop_targets();
        }
        if finish_drag {
            self.finish_drag(event_loop);
        }
    }

    fn render_main(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let drag_state = self.drag_state;
        let active_globe_bounds = self.active_globe_bounds();
        let now = Instant::now();
        let frame_start = now;

        let (delta, fps) = {
            let core = self.core.as_mut().expect("core ready");
            let delta = (now - core.last_frame).as_secs_f32();
            core.last_frame = now;
            let fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };
            (delta, fps)
        };

        let (world_ms, tile_ms, tile_bars) = {
            let core = self.core.as_mut().expect("core ready");
            let tile_viewport_size = active_globe_bounds
                .map(|bounds| (bounds.width, bounds.height))
                .unwrap_or_else(|| {
                    let (width, height) = core.renderer.size();
                    (width.max(1), height.max(1))
                });
            core.renderer
                .set_camera_aspect(tile_viewport_size.0, tile_viewport_size.1);
            let mut world_updated = false;
            let world_start = Instant::now();
            core.world_accum += delta.max(0.0);
            if core.world_accum >= core.world_update_interval {
                let steps = (core.world_accum / core.world_update_interval)
                    .floor()
                    .min(4.0) as u32;
                for _ in 0..steps {
                    core.world.update(core.world_update_interval);
                }
                core.world_accum -= steps as f32 * core.world_update_interval;
                world_updated = true;
                core.instances_dirty = true;
            }
            if world_updated || core.instances_dirty {
                core.world.collect_instances(&mut core.instances);
                filter_instances(&core.instances, &core.overlay_settings, &mut core.filtered_instances);
                core.instances_dirty = false;
                core.cull_dirty = true;
            }
            if core.cull_dirty {
                cull_instances_for_render(
                    &core.filtered_instances,
                    &core.renderer,
                    core.world.globe_radius(),
                    &mut core.render_instances,
                );
                core.renderer.update_instances(&core.render_instances);
                core.cull_dirty = false;
            }
            let world_ms = world_start.elapsed().as_secs_f32() * 1000.0;

            let tile_start = Instant::now();
            for result in core
                .tile_rx
                .try_iter()
                .take(MAX_TILE_UPLOADS_PER_FRAME)
            {
                core.tile_layers
                    .handle_result(&mut core.renderer, result, now);
            }
            core.tile_layers.update(
                &mut core.renderer,
                &mut core.tile_fetcher,
                &core.overlay_settings,
                now,
                &mut core.tile_request_id,
                tile_viewport_size,
            );
            let tile_bars = core.tile_layers.progress_bars();
            core.tile_layers.update_diagnostics(&mut core.diagnostics, now);
            let tile_ms = tile_start.elapsed().as_secs_f32() * 1000.0;

            (world_ms, tile_ms, tile_bars)
        };

        let (messages, viewport, ui_ms) = {
            let core = self.core.as_mut().expect("core ready");
            let main = self.main.as_mut().expect("main ready");
            let ui_start = Instant::now();
            let scale_factor = main.window.scale_factor() as f32;
            let physical_size = Size::new(core.renderer.size().0, core.renderer.size().1);
            let viewport = Viewport::with_physical_size(physical_size, scale_factor);
            let cursor = match main.cursor_logical {
                Some(point) => Cursor::Available(point),
                None => Cursor::Unavailable,
            };
            let events: Vec<IcedEvent> = main.ui_events.drain(..).collect();
            let mut messages = Vec::new();
            let drag_preview = drag_state
                .and_then(|state| {
                    main.cursor_logical.map(|cursor| DragPreview {
                        panel: state.panel,
                        cursor,
                    })
                });
            let drop_indicator = drag_state
                .and_then(|state| main.drop_target.then_some(state.panel))
                .and_then(|panel| main.cursor_logical.map(|cursor| (panel, cursor)))
                .and_then(|(panel, cursor)| drop_indicator_for_main(core, main, panel, cursor));
            let element = core.ui.view_main(
                main.window.id(),
                main.panels,
                &core.world,
                &core.renderer,
                &core.diagnostics,
                &tile_bars,
                main.drop_target,
                drag_preview,
                drop_indicator,
            );
            let mut user_interface = UserInterface::build(
                element,
                viewport.logical_size(),
                std::mem::take(&mut main.ui_cache),
                &mut main.ui_renderer,
            );
            let _ = user_interface.update(
                &events,
                cursor,
                &mut main.ui_renderer,
                &mut main.ui_clipboard,
                &mut messages,
            );
            user_interface.draw(
                &mut main.ui_renderer,
                &core.ui_theme,
                &renderer::Style::default(),
                cursor,
            );
            main.ui_cache = user_interface.into_cache();
            let ui_ms = ui_start.elapsed().as_secs_f32() * 1000.0;
            (messages, viewport, ui_ms)
        };

        self.process_ui_messages(event_loop, messages);

        let render_ms = {
            let core = self.core.as_mut().expect("core ready");
            let main = self.main.as_mut().expect("main ready");
            let render_start = Instant::now();
            let surface_texture = match core.renderer.begin_frame() {
                Ok(frame) => frame,
                Err(wgpu::SurfaceError::Outdated) => {
                    core.renderer.reconfigure();
                    return Ok(());
                }
                Err(wgpu::SurfaceError::Lost) => {
                    core.renderer.reconfigure();
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
            let depth_view = core.renderer.viewport_depth_view().clone();

            let mut encoder = core
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("c2-native encoder"),
                });
            let globe_bounds = globe_bounds_for_main(core, main);
            if let Some(bounds) = globe_bounds {
                core.renderer.set_camera_aspect(bounds.width, bounds.height);
                core.renderer
                    .render_scene(&mut encoder, &view, &depth_view, Some(bounds));
            } else {
                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("c2-native clear pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
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
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                std::mem::drop(pass);
            }
            core.renderer.queue.submit([encoder.finish()]);

            main.ui_renderer
                .present(None, core.renderer.surface_format(), &view, &viewport);
            surface_texture.present();
            render_start.elapsed().as_secs_f32() * 1000.0
        };

        let frame_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        if let Some(core) = self.core.as_mut() {
            core.diagnostics.perf = core.perf_stats.update(
                frame_ms,
                world_ms,
                tile_ms,
                ui_ms,
                render_ms,
                fps,
            );
        }

        Ok(())
    }

    fn render_detached(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
    ) -> anyhow::Result<()> {
        let drag_state = self.drag_state;
        let (messages, viewport, globe_active) = {
            let core = self.core.as_mut().expect("core ready");
            let Some(detached) = self.detached.get_mut(&window_id) else {
                return Ok(());
            };
            let scale_factor = detached.window.scale_factor() as f32;
            let physical_size = Size::new(detached.size.width, detached.size.height);
            let viewport = Viewport::with_physical_size(physical_size, scale_factor);
            let cursor = match detached.cursor_logical {
                Some(point) => Cursor::Available(point),
                None => Cursor::Unavailable,
            };
            let events: Vec<IcedEvent> = detached.ui_events.drain(..).collect();
            let mut messages = Vec::new();
            let tile_bars = core.tile_layers.progress_bars();
            let globe_active = detached.active_panel == PanelId::Globe;
            let drag_preview = drag_state
                .and_then(|state| {
                    detached.cursor_logical.map(|cursor| DragPreview {
                        panel: state.panel,
                        cursor,
                    })
                });
            let drop_indicator = drag_state
                .and_then(|state| detached.drop_target.then_some(state.panel))
                .and_then(|panel| detached.cursor_logical.map(|cursor| (panel, cursor)))
                .and_then(|(panel, cursor)| drop_indicator_for_detached(core, detached, panel, cursor));
            let element = core.ui.view_detached(
                detached.window.id(),
                &detached.panels,
                detached.active_panel,
                &core.world,
                &core.renderer,
                &core.diagnostics,
                &tile_bars,
                detached.drop_target,
                drag_preview,
                drop_indicator,
            );
            let mut user_interface = UserInterface::build(
                element,
                viewport.logical_size(),
                std::mem::take(&mut detached.ui_cache),
                &mut detached.ui_renderer,
            );
            let _ = user_interface.update(
                &events,
                cursor,
                &mut detached.ui_renderer,
                &mut detached.ui_clipboard,
                &mut messages,
            );
            user_interface.draw(
                &mut detached.ui_renderer,
                &core.ui_theme,
                &renderer::Style::default(),
                cursor,
            );
            detached.ui_cache = user_interface.into_cache();
            (messages, viewport, globe_active)
        };

        self.process_ui_messages(event_loop, messages);

        if !self.detached.contains_key(&window_id) {
            return Ok(());
        }

        let core = self.core.as_mut().expect("core ready");
        let detached = self.detached.get_mut(&window_id).expect("window ready");

        let surface_texture = match detached.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                detached
                    .surface
                    .configure(&core.renderer.device, &detached.surface_config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Lost) => {
                detached
                    .surface
                    .configure(&core.renderer.device, &detached.surface_config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                event_loop.exit();
                return Ok(());
            }
            Err(err) => {
                eprintln!("surface error: {err:?}");
                return Ok(());
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if globe_active {
            let mut encoder = core
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("c2-native dock encoder"),
                });
            let globe_bounds = globe_bounds_for_detached(core, detached);
            let (aspect_width, aspect_height) = globe_bounds
                .map(|bounds| (bounds.width, bounds.height))
                .unwrap_or((detached.size.width, detached.size.height));
            core.renderer.set_camera_aspect(aspect_width, aspect_height);
            core.renderer
                .render_scene(&mut encoder, &view, &detached.depth_view, globe_bounds);
            core.renderer.queue.submit([encoder.finish()]);
        }
        detached
            .ui_renderer
            .present(None, detached.surface_config.format, &view, &viewport);
        surface_texture.present();

        Ok(())
    }

    fn process_ui_messages(&mut self, event_loop: &ActiveEventLoop, messages: Vec<UiMessage>) {
        if messages.is_empty() {
            return;
        }

        if let Some(core) = self.core.as_mut() {
            for message in &messages {
                core.ui.update(message.clone());
            }
        }

        for message in messages {
            match message {
                UiMessage::StartDrag { panel, window } => {
                    self.start_drag(panel, window);
                }
                UiMessage::SelectTab { panel, window } => {
                    if let Some(detached) = self.detached.get_mut(&window) {
                        detached.active_panel = panel;
                        if panel != PanelId::Globe {
                            detached.globe_dragging = false;
                        }
                    }
                }
                UiMessage::DetachPanel { panel, window } => {
                    self.detach_panel(event_loop, panel, window);
                }
                UiMessage::DockBack { window } => {
                    self.dock_back(window);
                }
                _ => {}
            }
        }

        if let Some(core) = self.core.as_mut() {
            core.sync_operations_settings();
        }
    }

    fn start_drag(&mut self, panel: PanelId, window: WindowId) {
        self.drag_state = Some(DragState { panel, source_window: window });
        self.hovered_window = Some(window);
        self.update_drop_targets();
        self.set_drag_cursor(true);
    }

    fn finish_drag(&mut self, event_loop: &ActiveEventLoop) {
        let Some(drag_state) = self.drag_state.take() else {
            return;
        };

        let target_panel = self
            .core
            .as_ref()
            .and_then(|core| self.panel_at_cursor(core));

        let target = self.window_at_cursor();
        self.clear_drop_targets();

        match target {
            Some(target_window) if target_window != drag_state.source_window => {
                if let Some(target_panel) = target_panel {
                    if target_panel != drag_state.panel {
                        self.swap_panels(
                            drag_state.panel,
                            drag_state.source_window,
                            target_panel,
                            target_window,
                        );
                    } else {
                        self.move_panel_to_window(
                            drag_state.panel,
                            drag_state.source_window,
                            target_window,
                        );
                    }
                } else {
                    self.move_panel_to_window(
                        drag_state.panel,
                        drag_state.source_window,
                        target_window,
                    );
                }
            }
            Some(_) => {}
            None => {
                self.detach_panel(event_loop, drag_state.panel, drag_state.source_window);
            }
        }
        self.set_drag_cursor(false);
    }

    fn panel_at_cursor(&self, core: &AppCore) -> Option<PanelId> {
        let target = self.window_at_cursor()?;
        if let Some(main) = self.main.as_ref() {
            if main.window.id() == target {
                let cursor = main.cursor_logical?;
                return panel_at_point_main(core, main, cursor);
            }
        }
        let detached = self.detached.get(&target)?;
        let cursor = detached.cursor_logical?;
        panel_at_point_detached(core, detached, cursor)
    }

    fn swap_panels(
        &mut self,
        panel_a: PanelId,
        window_a: WindowId,
        panel_b: PanelId,
        window_b: WindowId,
    ) {
        self.remove_panel_from_window(panel_a, window_a);
        self.remove_panel_from_window(panel_b, window_b);
        self.add_panel_to_window(panel_a, window_b);
        self.add_panel_to_window(panel_b, window_a);
        self.cleanup_window(window_a);
        self.cleanup_window(window_b);
    }

    fn detach_panel(&mut self, event_loop: &ActiveEventLoop, panel: PanelId, source_window: WindowId) {
        let Some(core) = self.core.as_ref() else {
            return;
        };

        let layout = core.ui.layout();
        let size = detached_window_size(layout, panel);
        let position = self
            .global_cursor
            .map(|cursor| PhysicalPosition::new(cursor.x as i32, cursor.y as i32))
            .or_else(|| fallback_window_position(self.main.as_ref()));

        let mut attrs = WindowAttributes::default()
            .with_title("Dock Window")
            .with_inner_size(size);
        if let Some(position) = position {
            attrs = attrs.with_position(position);
        }

        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                eprintln!("failed to create dock window: {err:?}");
                return;
            }
        };

        let detached = match DetachedWindow::new(window.clone(), core, vec![panel]) {
            Ok(detached) => detached,
            Err(err) => {
                eprintln!("failed to initialize dock window: {err:?}");
                return;
            }
        };
        let window_id = window.id();
        self.detached.insert(window_id, detached);
        self.remove_panel_from_window(panel, source_window);
        self.cleanup_window(source_window);
    }

    fn dock_back(&mut self, window_id: WindowId) {
        let Some(detached) = self.detached.remove(&window_id) else {
            return;
        };
        if let Some(main) = self.main.as_mut() {
            for panel in detached.panels {
                add_panel_to_main(main, panel);
            }
        }
    }

    fn move_panel_to_window(&mut self, panel: PanelId, source: WindowId, target: WindowId) {
        self.remove_panel_from_window(panel, source);
        self.add_panel_to_window(panel, target);
        self.cleanup_window(source);
    }

    fn remove_panel_from_window(&mut self, panel: PanelId, window_id: WindowId) {
        if let Some(main) = self.main.as_mut() {
            if main.window.id() == window_id {
                remove_panel_from_main(main, panel);
                return;
            }
        }
        if let Some(detached) = self.detached.get_mut(&window_id) {
            remove_panel_from_detached(detached, panel);
        }
    }

    fn add_panel_to_window(&mut self, panel: PanelId, window_id: WindowId) {
        if let Some(main) = self.main.as_mut() {
            if main.window.id() == window_id {
                add_panel_to_main(main, panel);
                return;
            }
        }
        if let Some(detached) = self.detached.get_mut(&window_id) {
            add_panel_to_detached(detached, panel);
        }
    }

    fn cleanup_window(&mut self, window_id: WindowId) {
        let remove = self
            .detached
            .get(&window_id)
            .is_some_and(|detached| detached.panels.is_empty());
        if remove {
            self.detached.remove(&window_id);
            if self.hovered_window == Some(window_id) {
                self.hovered_window = None;
            }
        }
    }

    fn update_global_cursor(&mut self, window: &Window, position: PhysicalPosition<f64>) {
        if let Ok(inner_pos) = window.inner_position() {
            self.global_cursor = Some(PhysicalPosition::new(
                inner_pos.x as f64 + position.x,
                inner_pos.y as f64 + position.y,
            ));
        }
    }

    fn window_at_cursor(&self) -> Option<WindowId> {
        if let Some(cursor) = self.global_cursor {
            if let Some(main) = self.main.as_ref() {
                if window_contains(&main.window, cursor) {
                    return Some(main.window.id());
                }
            }
            for (id, window) in &self.detached {
                if window_contains(&window.window, cursor) {
                    return Some(*id);
                }
            }
        }
        self.hovered_window
    }

    fn update_drop_targets(&mut self) {
        let dragging = self.drag_state.is_some();
        if let Some(main) = self.main.as_mut() {
            main.drop_target = false;
        }
        for window in self.detached.values_mut() {
            window.drop_target = false;
        }
        if !dragging {
            return;
        }
        let Some(target) = self.window_at_cursor() else {
            return;
        };
        if let Some(main) = self.main.as_mut() {
            if main.window.id() == target {
                main.drop_target = true;
                return;
            }
        }
        if let Some(detached) = self.detached.get_mut(&target) {
            detached.drop_target = true;
        }
    }

    fn clear_drop_targets(&mut self) {
        if let Some(main) = self.main.as_mut() {
            main.drop_target = false;
        }
        for window in self.detached.values_mut() {
            window.drop_target = false;
        }
    }

    fn set_drag_cursor(&self, dragging: bool) {
        let icon = if dragging {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Default
        };
        if let Some(main) = self.main.as_ref() {
            let _ = main.window.set_cursor(icon);
        }
        for window in self.detached.values() {
            let _ = window.window.set_cursor(icon);
        }
    }

    fn active_globe_bounds(&self) -> Option<RenderBounds> {
        let core = self.core.as_ref()?;
        if let Some(main) = self.main.as_ref() {
            if main.panels.globe {
                return globe_bounds_for_main(core, main);
            }
        }
        for detached in self.detached.values() {
            if detached.active_panel == PanelId::Globe {
                if let Some(bounds) = globe_bounds_for_detached(core, detached) {
                    return Some(bounds);
                }
            }
        }
        None
    }
}

struct AppCore {
    renderer: Renderer,
    world: WorldState,
    ui: UiState,
    ui_theme: Theme,
    overlay_settings: OperationsState,
    last_frame: Instant,
    instances: Vec<RenderInstance>,
    filtered_instances: Vec<RenderInstance>,
    render_instances: Vec<RenderInstance>,
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
}

impl AppCore {
    fn new(mut renderer: Renderer) -> anyhow::Result<Self> {
        let world = WorldState::seeded();
        let ui = UiState::new();
        let overlay_settings = ui.operations().clone();
        let (tile_fetcher, tile_rx) = TileFetcher::new(6);
        let mut tile_layers = TileLayers::new();
        let mut tile_request_id = 0;
        tile_layers.apply_settings(
            &overlay_settings,
            &mut renderer,
            &tile_fetcher,
            &mut tile_request_id,
        );
        let (map_opacity, sea_opacity, weather_opacity) =
            tile_layers.overlay_opacities(&overlay_settings);
        renderer.update_overlay(
            if overlay_settings.show_base { 1.0 } else { 0.0 },
            map_opacity,
            sea_opacity,
            weather_opacity,
        );

        Ok(Self {
            renderer,
            world,
            ui,
            ui_theme: Theme::Dark,
            overlay_settings,
            last_frame: Instant::now(),
            instances: Vec::new(),
            filtered_instances: Vec::new(),
            render_instances: Vec::new(),
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
        })
    }

    fn sync_operations_settings(&mut self) {
        let new_settings = self.ui.operations().clone();
        if new_settings != self.overlay_settings {
            self.overlay_settings = new_settings;
            filter_instances(&self.instances, &self.overlay_settings, &mut self.filtered_instances);
            self.instances_dirty = false;
            self.cull_dirty = true;
            let (map_opacity, sea_opacity, weather_opacity) =
                self.tile_layers.overlay_opacities(&self.overlay_settings);
            self.renderer.update_overlay(
                if self.overlay_settings.show_base { 1.0 } else { 0.0 },
                map_opacity,
                sea_opacity,
                weather_opacity,
            );
            self.tile_layers.apply_settings(
                &self.overlay_settings,
                &mut self.renderer,
                &self.tile_fetcher,
                &mut self.tile_request_id,
            );
        }
    }
}

struct MainWindow {
    window: Arc<Window>,
    ui_cache: Cache,
    ui_renderer: iced_wgpu::Renderer,
    ui_events: Vec<IcedEvent>,
    ui_clipboard: Clipboard,
    cursor_logical: Option<Point>,
    last_cursor_physical: Option<PhysicalPosition<f64>>,
    globe_dragging: bool,
    panels: MainPanels,
    drop_target: bool,
}

impl MainWindow {
    fn new(window: Arc<Window>, ui_renderer: iced_wgpu::Renderer) -> Self {
        Self {
            window: window.clone(),
            ui_cache: Cache::new(),
            ui_renderer,
            ui_events: Vec::new(),
            ui_clipboard: Clipboard::connect(window),
            cursor_logical: None,
            last_cursor_physical: None,
            globe_dragging: false,
            panels: MainPanels {
                globe: true,
                operations: true,
                entities: true,
                inspector: true,
            },
            drop_target: false,
        }
    }
}

struct DetachedWindow {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    ui_cache: Cache,
    ui_renderer: iced_wgpu::Renderer,
    ui_events: Vec<IcedEvent>,
    ui_clipboard: Clipboard,
    cursor_logical: Option<Point>,
    last_cursor_physical: Option<PhysicalPosition<f64>>,
    globe_dragging: bool,
    panels: Vec<PanelId>,
    active_panel: PanelId,
    drop_target: bool,
}

impl DetachedWindow {
    fn new(window: Arc<Window>, core: &AppCore, mut panels: Vec<PanelId>) -> anyhow::Result<Self> {
        if panels.is_empty() {
            panels.push(PanelId::Operations);
        }
        panels.sort_by_key(|panel| panel.order());
        let active_panel = panels[0];
        let size = window.inner_size();
        let surface = core.renderer.create_surface(window.as_ref())?;
        let caps = surface.get_capabilities(core.renderer.adapter());
        let format = if caps.formats.contains(&core.renderer.surface_format()) {
            core.renderer.surface_format()
        } else {
            caps.formats[0]
        };
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&core.renderer.device, &surface_config);
        let ui_renderer = build_ui_renderer(&core.renderer, format);
        let (depth_texture, depth_view) =
            create_depth_texture(&core.renderer.device, surface_config.width, surface_config.height);

        Ok(Self {
            window: window.clone(),
            surface,
            surface_config,
            size,
            depth_texture,
            depth_view,
            ui_cache: Cache::new(),
            ui_renderer,
            ui_events: Vec::new(),
            ui_clipboard: Clipboard::connect(window),
            cursor_logical: None,
            last_cursor_physical: None,
            globe_dragging: false,
            panels,
            active_panel,
            drop_target: false,
        })
    }
}

#[derive(Clone, Copy)]
struct DragState {
    panel: PanelId,
    source_window: WindowId,
}

fn build_ui_renderer(renderer: &Renderer, format: wgpu::TextureFormat) -> iced_wgpu::Renderer {
    let ui_engine = Engine::new(
        renderer.adapter(),
        renderer.device.clone(),
        renderer.queue.clone(),
        format,
        None,
        Shell::headless(),
    );
    iced_wgpu::Renderer::new(ui_engine, iced::Font::default(), 14.0.into())
}

fn cursor_in_globe(core: &AppCore, main: &MainWindow) -> bool {
    if !main.panels.globe {
        return false;
    }
    let Some(cursor) = main.cursor_logical else {
        return false;
    };
    let scale_factor = main.window.scale_factor() as f32;
    let size = core.renderer.size();
    let logical_size = Size::new(
        size.0 as f32 / scale_factor,
        size.1 as f32 / scale_factor,
    );
    let globe_rect = core.ui.globe_rect(logical_size, main.panels);
    globe_rect.contains(cursor)
}

fn cursor_in_detached_globe(core: &AppCore, detached: &DetachedWindow) -> bool {
    if detached.active_panel != PanelId::Globe {
        return false;
    }
    let Some(cursor) = detached.cursor_logical else {
        return false;
    };
    let scale_factor = detached.window.scale_factor() as f32;
    let logical_size = Size::new(
        detached.size.width as f32 / scale_factor,
        detached.size.height as f32 / scale_factor,
    );
    let has_tabs = detached.panels.len() > 1;
    let globe_rect = core.ui.detached_globe_rect(logical_size, has_tabs);
    globe_rect.contains(cursor)
}

fn globe_bounds_for_main(core: &AppCore, main: &MainWindow) -> Option<RenderBounds> {
    if !main.panels.globe {
        return None;
    }
    let scale_factor = main.window.scale_factor() as f32;
    let size = core.renderer.size();
    let logical_size = Size::new(
        size.0 as f32 / scale_factor,
        size.1 as f32 / scale_factor,
    );
    let rect = core.ui.globe_rect(logical_size, main.panels);
    render_bounds_from_rect(rect, scale_factor, size)
}

fn globe_bounds_for_detached(core: &AppCore, detached: &DetachedWindow) -> Option<RenderBounds> {
    if detached.active_panel != PanelId::Globe {
        return None;
    }
    let scale_factor = detached.window.scale_factor() as f32;
    let logical_size = Size::new(
        detached.size.width as f32 / scale_factor,
        detached.size.height as f32 / scale_factor,
    );
    let has_tabs = detached.panels.len() > 1;
    let rect = core.ui.detached_globe_rect(logical_size, has_tabs);
    render_bounds_from_rect(rect, scale_factor, (detached.size.width, detached.size.height))
}

fn render_bounds_from_rect(
    rect: Rectangle,
    scale_factor: f32,
    max_size: (u32, u32),
) -> Option<RenderBounds> {
    if rect.width <= 1.0 || rect.height <= 1.0 {
        return None;
    }
    let mut x = (rect.x * scale_factor).floor() as i32;
    let mut y = (rect.y * scale_factor).floor() as i32;
    let mut width = (rect.width * scale_factor).floor() as i32;
    let mut height = (rect.height * scale_factor).floor() as i32;

    let max_w = max_size.0 as i32;
    let max_h = max_size.1 as i32;

    if x < 0 {
        width += x;
        x = 0;
    }
    if y < 0 {
        height += y;
        y = 0;
    }
    if x >= max_w || y >= max_h {
        return None;
    }
    if x + width > max_w {
        width = max_w - x;
    }
    if y + height > max_h {
        height = max_h - y;
    }
    if width <= 1 || height <= 1 {
        return None;
    }

    Some(RenderBounds {
        x: x as u32,
        y: y as u32,
        width: width as u32,
        height: height as u32,
    })
}

fn resize_detached_window(core: &AppCore, detached: &mut DetachedWindow, size: PhysicalSize<u32>) {
    let width = size.width.max(1);
    let height = size.height.max(1);
    detached.size = PhysicalSize::new(width, height);
    detached.surface_config.width = width;
    detached.surface_config.height = height;
    detached
        .surface
        .configure(&core.renderer.device, &detached.surface_config);
    let (depth_texture, depth_view) = create_depth_texture(&core.renderer.device, width, height);
    detached.depth_texture = depth_texture;
    detached.depth_view = depth_view;
}

fn add_panel_to_main(main: &mut MainWindow, panel: PanelId) {
    match panel {
        PanelId::Globe => main.panels.globe = true,
        PanelId::Operations => main.panels.operations = true,
        PanelId::Entities => main.panels.entities = true,
        PanelId::Inspector => main.panels.inspector = true,
    }
}

fn remove_panel_from_main(main: &mut MainWindow, panel: PanelId) {
    match panel {
        PanelId::Globe => main.panels.globe = false,
        PanelId::Operations => main.panels.operations = false,
        PanelId::Entities => main.panels.entities = false,
        PanelId::Inspector => main.panels.inspector = false,
    }
}

fn add_panel_to_detached(detached: &mut DetachedWindow, panel: PanelId) {
    if !detached.panels.contains(&panel) {
        detached.panels.push(panel);
        detached.panels.sort_by_key(|panel| panel.order());
    }
    detached.active_panel = panel;
}

fn remove_panel_from_detached(detached: &mut DetachedWindow, panel: PanelId) {
    detached.panels.retain(|entry| *entry != panel);
    if detached.active_panel == panel {
        detached.active_panel = detached
            .panels
            .first()
            .copied()
            .unwrap_or(PanelId::Operations);
    }
}

fn window_contains(window: &Window, cursor: PhysicalPosition<f64>) -> bool {
    let Ok(position) = window.inner_position() else {
        return false;
    };
    let size = window.inner_size();
    let left = position.x as f64;
    let top = position.y as f64;
    let right = left + size.width as f64;
    let bottom = top + size.height as f64;
    cursor.x >= left && cursor.x <= right && cursor.y >= top && cursor.y <= bottom
}

fn drop_indicator_for_main(
    core: &AppCore,
    main: &MainWindow,
    panel: PanelId,
    cursor: Point,
) -> Option<DropIndicator> {
    let scale_factor = main.window.scale_factor() as f32;
    let size = core.renderer.size();
    let logical_size = Size::new(
        size.0 as f32 / scale_factor,
        size.1 as f32 / scale_factor,
    );
    let layout = core.ui.layout();
    if let Some(target_panel) = panel_at_point_main(core, main, cursor) {
        let rect = main_panel_rect(layout, logical_size, main.panels, target_panel)?;
        return Some(DropIndicator { rect });
    }
    let mut panels = main.panels;
    match panel {
        PanelId::Globe => panels.globe = true,
        PanelId::Operations => panels.operations = true,
        PanelId::Entities => panels.entities = true,
        PanelId::Inspector => panels.inspector = true,
    }
    let rect = main_panel_rect(layout, logical_size, panels, panel)?;
    Some(DropIndicator { rect })
}

fn drop_indicator_for_detached(
    core: &AppCore,
    detached: &DetachedWindow,
    _panel: PanelId,
    cursor: Point,
) -> Option<DropIndicator> {
    let scale_factor = detached.window.scale_factor() as f32;
    let logical_size = Size::new(
        detached.size.width as f32 / scale_factor,
        detached.size.height as f32 / scale_factor,
    );
    let layout = core.ui.layout();
    if detached.panels.len() > 1 {
        if let Some(tab_rect) = detached_tab_drop_rect(layout, logical_size) {
            if tab_rect.contains(cursor) {
                return Some(DropIndicator { rect: tab_rect });
            }
        }
    }
    let has_tabs = detached.panels.len() > 1;
    let rect = core.ui.detached_globe_rect(logical_size, has_tabs);
    Some(DropIndicator { rect })
}

fn main_panel_rect(
    layout: crate::ui::UiLayout,
    window_size: Size,
    panels: MainPanels,
    panel: PanelId,
) -> Option<Rectangle> {
    let content_width = (window_size.width - 2.0 * layout.outer_padding).max(0.0);
    let content_height = (window_size.height - 2.0 * layout.outer_padding).max(0.0);
    if content_width <= 1.0 || content_height <= 1.0 {
        return None;
    }
    let header_height = layout.top_bar_height;
    let inspector_height = if panels.inspector {
        layout.inspector_height
    } else {
        0.0
    };
    let top_spacing = layout.column_spacing;
    let bottom_spacing = if panels.inspector {
        layout.column_spacing
    } else {
        0.0
    };
    let row_height =
        (content_height - header_height - top_spacing - inspector_height - bottom_spacing).max(0.0);
    let row_y = layout.outer_padding + header_height + top_spacing;
    let row_x = layout.outer_padding;

    let operations_present = panels.operations;
    let entities_present = panels.entities;
    let operations_width = if operations_present { layout.panel_width } else { 0.0 };
    let entities_width = if entities_present { layout.panel_width } else { 0.0 };
    let operations_gap = if operations_present { layout.row_spacing } else { 0.0 };
    let entities_gap = if entities_present { layout.row_spacing } else { 0.0 };
    let globe_width =
        (content_width - operations_width - entities_width - operations_gap - entities_gap).max(0.0);
    let globe_x = row_x + operations_width + operations_gap;
    let entities_x = globe_x + globe_width + entities_gap;

    let rect = match panel {
        PanelId::Operations => Rectangle::new(
            Point::new(row_x, row_y),
            Size::new(operations_width, row_height),
        ),
        PanelId::Globe => Rectangle::new(
            Point::new(globe_x, row_y),
            Size::new(globe_width, row_height),
        ),
        PanelId::Entities => Rectangle::new(
            Point::new(entities_x, row_y),
            Size::new(entities_width, row_height),
        ),
        PanelId::Inspector => Rectangle::new(
            Point::new(row_x, row_y + row_height + bottom_spacing),
            Size::new(content_width, inspector_height),
        ),
    };
    Some(rect)
}

fn detached_tab_drop_rect(layout: crate::ui::UiLayout, window_size: Size) -> Option<Rectangle> {
    let content_width = (window_size.width - 2.0 * layout.outer_padding).max(0.0);
    let content_height = (window_size.height - 2.0 * layout.outer_padding).max(0.0);
    if content_width <= 1.0 || content_height <= 1.0 {
        return None;
    }
    let x = layout.outer_padding;
    let y = layout.outer_padding + layout.top_bar_height + layout.column_spacing;
    Some(Rectangle::new(
        Point::new(x, y),
        Size::new(content_width, layout.tab_bar_height),
    ))
}

fn panel_at_point_main(core: &AppCore, main: &MainWindow, cursor: Point) -> Option<PanelId> {
    let scale_factor = main.window.scale_factor() as f32;
    let size = core.renderer.size();
    let logical_size = Size::new(
        size.0 as f32 / scale_factor,
        size.1 as f32 / scale_factor,
    );
    let layout = core.ui.layout();
    for panel in PanelId::ALL {
        if !main.panels.contains(panel) {
            continue;
        }
        let rect = main_panel_rect(layout, logical_size, main.panels, panel)?;
        if rect.contains(cursor) {
            return Some(panel);
        }
    }
    None
}

fn panel_at_point_detached(
    core: &AppCore,
    detached: &DetachedWindow,
    cursor: Point,
) -> Option<PanelId> {
    let scale_factor = detached.window.scale_factor() as f32;
    let logical_size = Size::new(
        detached.size.width as f32 / scale_factor,
        detached.size.height as f32 / scale_factor,
    );
    let layout = core.ui.layout();
    if detached.panels.len() > 1 {
        if let Some(tab_rect) = detached_tab_drop_rect(layout, logical_size) {
            if tab_rect.contains(cursor) {
                return None;
            }
        }
    }
    let has_tabs = detached.panels.len() > 1;
    let body_rect = core.ui.detached_globe_rect(logical_size, has_tabs);
    if body_rect.contains(cursor) {
        return Some(detached.active_panel);
    }
    None
}

fn detached_window_size(layout: crate::ui::UiLayout, panel: PanelId) -> winit::dpi::LogicalSize<f64> {
    let base_width = layout.panel_width + layout.outer_padding * 2.0 + 80.0;
    let base_height = match panel {
        PanelId::Globe => 520.0,
        PanelId::Operations => 560.0,
        PanelId::Entities => 420.0,
        PanelId::Inspector => layout.inspector_height + layout.outer_padding * 2.0 + 220.0,
    };
    winit::dpi::LogicalSize::new(base_width as f64, base_height as f64)
}

fn fallback_window_position(main: Option<&MainWindow>) -> Option<PhysicalPosition<i32>> {
    let main = main?;
    let pos = main.window.inner_position().ok()?;
    Some(PhysicalPosition::new(pos.x + 40, pos.y + 40))
}

fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("detached depth"),
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
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
        viewport_size: (u32, u32),
    ) {
        let provider = tile_provider_config(&settings.tile_provider);
        self.map.update(
            renderer,
            fetcher,
            now,
            request_id,
            provider,
            settings,
            viewport_size,
        );
        self.weather
            .update(renderer, fetcher, now, request_id, provider, settings, viewport_size);
        self.sea
            .update(renderer, fetcher, now, request_id, provider, settings, viewport_size);
    }

    fn handle_result(&mut self, renderer: &mut Renderer, result: TileResult, now: Instant) {
        match result.kind {
            TileKind::Base => self.map.handle_result(renderer, result, now),
            TileKind::Weather => self.weather.handle_result(renderer, result, now),
            TileKind::Sea => self.sea.handle_result(renderer, result, now),
        }
    }

    fn progress_bars(&self) -> Vec<TileBar> {
        vec![
            self.map
                .progress_bar("Map", iced::Color::from_rgb8(86, 156, 255)),
            self.sea
                .progress_bar("Sea", iced::Color::from_rgb8(64, 196, 196)),
            self.weather
                .progress_bar("Weather", iced::Color::from_rgb8(255, 164, 72)),
        ]
    }

    fn update_diagnostics(&self, diagnostics: &mut Diagnostics, now: Instant) {
        diagnostics.map = self.map.stats(now);
        diagnostics.weather = self.weather.stats(now);
        diagnostics.sea = self.sea.stats(now);
    }

    fn overlay_opacities(&self, settings: &OperationsState) -> (f32, f32, f32) {
        let map_opacity = if settings.show_map { self.map.opacity } else { 0.0 };
        let sea_opacity = if settings.show_sea { self.sea.opacity } else { 0.0 };
        let weather_opacity = if settings.show_weather {
            self.weather.opacity
        } else {
            0.0
        };
        (map_opacity, sea_opacity, weather_opacity)
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
    last_direction: glam::Vec3,
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
            last_direction: glam::Vec3::ZERO,
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
        viewport_size: (u32, u32),
    ) {
        if !self.enabled {
            return;
        }

        let (min_zoom, max_zoom, desired_zoom) = match self.kind {
            TileKind::Base => (
                provider.min_zoom,
                provider.max_zoom,
                pick_tile_zoom(renderer, viewport_size, provider, DEFAULT_GLOBE_RADIUS),
            ),
            TileKind::Weather => (
                WEATHER_MIN_ZOOM,
                WEATHER_MAX_ZOOM,
                pick_overlay_zoom(
                    renderer,
                    viewport_size,
                    WEATHER_MIN_ZOOM,
                    WEATHER_MAX_ZOOM,
                    DEFAULT_GLOBE_RADIUS,
                ),
            ),
            TileKind::Sea => (
                SEA_MIN_ZOOM,
                SEA_MAX_ZOOM,
                pick_overlay_zoom(
                    renderer,
                    viewport_size,
                    SEA_MIN_ZOOM,
                    SEA_MAX_ZOOM,
                    DEFAULT_GLOBE_RADIUS,
                ),
            ),
        };
        let mut desired_zoom = desired_zoom.clamp(min_zoom, max_zoom);
        let mut selection = compute_visible_tiles(
            renderer,
            viewport_size,
            desired_zoom,
            self.max_tiles,
        );
        while selection.total > self.max_tiles && desired_zoom > min_zoom {
            desired_zoom = desired_zoom.saturating_sub(1);
            selection = compute_visible_tiles(
                renderer,
                viewport_size,
                desired_zoom,
                self.max_tiles,
            );
        }
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

        let desired = selection.keys;
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
                provider_url: (self.kind == TileKind::Base && !provider.url.is_empty())
                    .then(|| provider.url.to_string()),
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

    fn progress_bar(&self, label: &'static str, color: iced::Color) -> TileBar {
        let pending = self.pending.len();
        let loaded = self.progress_loaded;
        let total = self.progress_total.max(loaded + pending);
        let has_work = self.enabled && total > 0;
        let progress = has_work.then(|| loaded as f32 / total as f32);
        TileBar {
            label,
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
        entries.sort_by(|a, b| match (a.1, b.1) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.2.cmp(&b.2),
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

struct TileSelection {
    keys: Vec<TileKey>,
    total: usize,
}

fn filter_instances(
    instances: &[RenderInstance],
    settings: &OperationsState,
    out: &mut Vec<RenderInstance>,
) {
    out.clear();
    out.extend(instances.iter().copied().filter(|instance| match instance.category {
        KIND_FLIGHT => settings.show_flights,
        KIND_SHIP => settings.show_ships,
        KIND_SATELLITE => settings.show_satellites,
        _ => true,
    }));
}

fn cull_instances_for_render(
    instances: &[RenderInstance],
    renderer: &Renderer,
    globe_radius: f32,
    out: &mut Vec<RenderInstance>,
) {
    out.clear();
    out.reserve(instances.len().min(20_000));
    let camera_pos = renderer.camera_position();
    let view_proj = renderer.view_proj();
    for instance in instances {
        let dist = instance.position.distance(camera_pos);
        if dist > globe_radius * 6.0 {
            continue;
        }
        let to_instance = instance.position - camera_pos;
        let to_instance_len = to_instance.length();
        if to_instance_len > 0.001 {
            let dir = to_instance / to_instance_len;
            if let Some(hit) = ray_sphere_intersect(camera_pos, dir, globe_radius) {
                if hit + 0.05 < to_instance_len {
                    continue;
                }
            }
        }
        let clip = view_proj * instance.position.extend(1.0);
        if clip.w.abs() <= f32::EPSILON {
            continue;
        }
        let ndc = clip.truncate() / clip.w;
        if ndc.z < -1.2 || ndc.z > 1.2 {
            continue;
        }
        out.push(*instance);
    }
}

fn pick_tile_zoom(
    renderer: &Renderer,
    viewport_size: (u32, u32),
    provider: TileProviderConfig,
    globe_radius: f32,
) -> u8 {
    pick_zoom(
        renderer,
        viewport_size,
        globe_radius,
        provider.min_zoom,
        provider.max_zoom,
        provider.zoom_bias,
    )
}

fn pick_overlay_zoom(
    renderer: &Renderer,
    viewport_size: (u32, u32),
    min_zoom: u8,
    max_zoom: u8,
    globe_radius: f32,
) -> u8 {
    pick_zoom(renderer, viewport_size, globe_radius, min_zoom, max_zoom, 0)
}

fn pick_zoom(
    renderer: &Renderer,
    viewport_size: (u32, u32),
    globe_radius: f32,
    min_zoom: u8,
    max_zoom: u8,
    zoom_bias: i8,
) -> u8 {
    let (width, height) = viewport_size;
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
    let tile_deg_width = deg_width * (crate::tiles::TILE_SIZE as f32 / width as f32);
    let tile_deg_height = deg_height * (crate::tiles::TILE_SIZE as f32 / height as f32);
    let tile_deg = tile_deg_width.max(tile_deg_height).max(0.0001);
    let mut zoom = (360.0 / tile_deg).log2().round() as i32 + zoom_bias as i32;
    let min_zoom = min_zoom as i32;
    let max_zoom = max_zoom as i32;
    zoom = zoom.clamp(min_zoom, max_zoom);
    zoom as u8
}

fn tile_bounds(key: TileKey) -> TileBounds {
    let tiles = 1u32 << key.zoom;
    let scale = tiles as f32;
    let lon_min = key.x as f32 / scale * 360.0 - 180.0;
    let lon_max = (key.x as f32 + 1.0) / scale * 360.0 - 180.0;
    let merc_min = key.y as f32 / scale;
    let merc_max = (key.y as f32 + 1.0) / scale;
    TileBounds {
        lon_min,
        lon_max,
        merc_min,
        merc_max,
    }
}

fn pick_focus_box_px(width: u32, height: u32, zoom: u8) -> f32 {
    let base = width.min(height) as f32;
    if base <= 0.0 {
        return 0.0;
    }
    let max_radius = (base * 0.32).max(220.0);
    let min_radius = (base * 0.2).max(140.0);
    let ratio = (zoom as f32 / 6.0).clamp(0.0, 1.0);
    min_radius + (max_radius - min_radius) * ratio
}

fn compute_visible_tiles(
    renderer: &Renderer,
    viewport_size: (u32, u32),
    zoom: u8,
    max_tiles: usize,
) -> TileSelection {
    let (width, height) = viewport_size;
    if width == 0 || height == 0 {
        return TileSelection {
            keys: Vec::new(),
            total: 0,
        };
    }
    let Some(center) = sample_geo(renderer, 0.0, 0.0, DEFAULT_GLOBE_RADIUS) else {
        return TileSelection {
            keys: Vec::new(),
            total: 0,
        };
    };
    let mut candidates = Vec::new();
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
        return TileSelection {
            keys: Vec::new(),
            total: 0,
        };
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
    let (lon_min, lon_max) = lon_range.unwrap_or((-180.0, 180.0));
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

    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
    let total = candidates.len();
    let keys = candidates
        .into_iter()
        .take(max_tiles)
        .map(|(key, _)| key)
        .collect();
    TileSelection { keys, total }
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

fn sample_geo(renderer: &Renderer, ndc_x: f32, ndc_y: f32, radius: f32) -> Option<GeoSample> {
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

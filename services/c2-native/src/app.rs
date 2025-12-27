use std::sync::Arc;
use std::time::Instant;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes};

use crate::ecs::{RenderInstance, WorldState};
use crate::renderer::Renderer;
use crate::ui::UiState;

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
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    last_frame: Instant,
    globe_texture_id: Option<egui::TextureId>,
    globe_dragging: bool,
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

        Ok(Self {
            window,
            renderer,
            world,
            ui,
            instances: Vec::new(),
            egui_ctx,
            egui_state,
            egui_renderer,
            last_frame: Instant::now(),
            globe_texture_id: None,
            globe_dragging: false,
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

        self.world.update(delta);
        self.instances.clear();
        self.world.collect_instances(&mut self.instances);
        self.renderer.update_instances(&self.instances);

        let raw_input = self.egui_state.take_egui_input(window);
        let output = self.egui_ctx.run(raw_input, |ctx| {
            self.ui.show(
                ctx,
                &self.world,
                &self.renderer,
                &self.instances,
                self.globe_texture_id,
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

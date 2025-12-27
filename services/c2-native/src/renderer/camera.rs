use glam::{Mat4, Vec3};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

pub struct Camera {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub aspect: f32,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(aspect: f32, distance: f32) -> Self {
        Self {
            distance,
            yaw: 0.4,
            pitch: 0.3,
            aspect,
            fov_y: 45.0_f32.to_radians(),
            near: 0.1,
            far: 2000.0,
        }
    }

    pub fn view_proj(&self) -> Mat4 {
        let position = self.position();
        let view = Mat4::look_at_rh(position, Vec3::ZERO, Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far);
        proj * view
    }

    pub fn position(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();
        Vec3::new(
            self.distance * cos_pitch * self.yaw.cos(),
            self.distance * self.pitch.sin(),
            self.distance * cos_pitch * self.yaw.sin(),
        )
    }

    pub fn update_aspect(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height.max(1) as f32;
    }
}

pub struct CameraController {
    rotate_sensitivity: f32,
    zoom_sensitivity: f32,
    dragging: bool,
    last_cursor: (f32, f32),
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            rotate_sensitivity: 0.006,
            zoom_sensitivity: 0.1,
            dragging: false,
            last_cursor: (0.0, 0.0),
        }
    }

    pub fn process_event(&mut self, event: &WindowEvent, camera: &mut Camera) {
        match event {
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    self.dragging = *state == ElementState::Pressed;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);
                if self.dragging {
                    let dx = x - self.last_cursor.0;
                    let dy = y - self.last_cursor.1;
                    camera.yaw += dx * self.rotate_sensitivity;
                    camera.pitch = (camera.pitch - dy * self.rotate_sensitivity)
                        .clamp(-1.45, 1.45);
                }
                self.last_cursor = (x, y);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                camera.distance = (camera.distance * (1.0 - scroll * self.zoom_sensitivity))
                    .clamp(60.0, 600.0);
            }
            _ => {}
        }
    }
}

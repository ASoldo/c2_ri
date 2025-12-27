use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use glam::{Vec2, Vec3, Vec4};

use crate::ecs::{RenderInstance, WorldState};
use crate::renderer::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockTab {
    Globe,
    Operations,
    Entities,
    Inspector,
}

pub struct UiState {
    dock_state: DockState<DockTab>,
    globe_rect: Option<egui::Rect>,
}

impl UiState {
    pub fn new() -> Self {
        let mut dock_state = DockState::new(vec![DockTab::Globe]);
        let root = NodeIndex::root();
        let [left, globe] = dock_state
            .main_surface_mut()
            .split_left(root, 0.28, vec![DockTab::Operations]);
        dock_state
            .main_surface_mut()
            .split_below(left, 0.35, vec![DockTab::Entities]);
        dock_state
            .main_surface_mut()
            .split_right(globe, 0.28, vec![DockTab::Inspector]);
        Self {
            dock_state,
            globe_rect: None,
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        world: &WorldState,
        renderer: &Renderer,
        instances: &[RenderInstance],
        globe_texture_id: Option<egui::TextureId>,
    ) {
        self.globe_rect = None;
        egui::TopBottomPanel::top("c2-topbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("C2 Walaris");
                ui.separator();
                ui.label(format!("Entities: {}", world.entity_count()));
                ui.label(format!(
                    "Viewport: {}x{}",
                    renderer.size().0,
                    renderer.size().1
                ));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut viewer = DockViewer {
                world,
                renderer,
                globe_rect: &mut self.globe_rect,
                globe_texture_id,
            };
            let style = Style::from_egui(ui.style());
            DockArea::new(&mut self.dock_state)
                .style(style)
                .show_inside(ui, &mut viewer);
        });

        self.draw_edge_compass(ctx, renderer, instances);
    }

    pub fn globe_rect(&self) -> Option<egui::Rect> {
        self.globe_rect
    }

    fn draw_edge_compass(
        &self,
        ctx: &egui::Context,
        renderer: &Renderer,
        instances: &[RenderInstance],
    ) {
        let Some(rect) = self.globe_rect else {
            return;
        };
        let bounds = rect.shrink(16.0);
        let view_proj = renderer.view_proj();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("edge-compass"),
        ));

        let max_indicators = 128usize;
        for instance in instances.iter().take(max_indicators) {
            let clip = view_proj * instance.position.extend(1.0);
            if clip.w == 0.0 {
                continue;
            }
            let mut ndc = Vec3::new(clip.x, clip.y, clip.z) / clip.w;
            let behind = clip.w < 0.0;
            if behind {
                ndc = -ndc;
            }
            let on_screen = !behind
                && ndc.x >= -1.0
                && ndc.x <= 1.0
                && ndc.y >= -1.0
                && ndc.y <= 1.0;
            if on_screen {
                continue;
            }

            let dir = Vec2::new(ndc.x, ndc.y);
            let max_comp = dir.x.abs().max(dir.y.abs());
            if max_comp <= f32::EPSILON {
                continue;
            }
            let edge = dir / max_comp;
            let t_x = (edge.x * 0.5 + 0.5).clamp(0.0, 1.0);
            let t_y = (1.0 - (edge.y * 0.5 + 0.5)).clamp(0.0, 1.0);
            let pos = egui::pos2(
                egui::lerp(bounds.left()..=bounds.right(), t_x),
                egui::lerp(bounds.top()..=bounds.bottom(), t_y),
            );

            let color = egui_color_from_rgba(instance.color);
            painter.circle_filled(pos, 10.0, color);
            painter.circle_stroke(
                pos,
                10.0,
                egui::Stroke::new(1.0, egui::Color32::from_white_alpha(160)),
            );
            let label = kind_label(instance.kind);
            let text_pos = egui::pos2(pos.x, pos.y - 16.0);
            painter.text(
                text_pos,
                egui::Align2::CENTER_BOTTOM,
                label,
                egui::FontId::monospace(10.0),
                egui::Color32::WHITE,
            );
        }
    }
}

fn egui_color_from_rgba(color: [f32; 4]) -> egui::Color32 {
    let rgba = Vec4::from_array(color).clamp(Vec4::ZERO, Vec4::ONE);
    egui::Color32::from_rgba_unmultiplied(
        (rgba.x * 255.0) as u8,
        (rgba.y * 255.0) as u8,
        (rgba.z * 255.0) as u8,
        (rgba.w * 255.0) as u8,
    )
}

fn kind_label(kind: u32) -> &'static str {
    match kind {
        0 => "FLT",
        1 => "SHP",
        2 => "SAT",
        _ => "UNK",
    }
}

struct DockViewer<'a> {
    world: &'a WorldState,
    renderer: &'a Renderer,
    globe_rect: &'a mut Option<egui::Rect>,
    globe_texture_id: Option<egui::TextureId>,
}

impl TabViewer for DockViewer<'_> {
    type Tab = DockTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            DockTab::Globe => "Globe".into(),
            DockTab::Operations => "Operations".into(),
            DockTab::Entities => "Entities".into(),
            DockTab::Inspector => "Inspector".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            DockTab::Globe => {
                let available = ui.available_size();
                let (rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
                *self.globe_rect = Some(rect);
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(6, 8, 12));
                if let Some(texture_id) = self.globe_texture_id {
                    let image = egui::Image::new((texture_id, rect.size()));
                    ui.put(rect, image);
                } else {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Globe loading...",
                        egui::FontId::proportional(14.0),
                        egui::Color32::from_gray(180),
                    );
                }
            }
            DockTab::Operations => {
                ui.heading("Operations Menu");
                ui.separator();
                ui.label("Globe layers, missions, and overlays will live here.");
                ui.add_space(8.0);
                ui.label("Status: connected to ECS runtime.");
            }
            DockTab::Entities => {
                ui.heading("Entities");
                ui.separator();
                ui.label(format!("Total entities: {}", self.world.entity_count()));
                ui.label("Filters and tasking controls will appear here.");
            }
            DockTab::Inspector => {
                ui.heading("Inspector");
                ui.separator();
                ui.label(format!(
                    "Render targets: {}x{}",
                    self.renderer.size().0,
                    self.renderer.size().1
                ));
                ui.label("Selection details will be shown here.");
            }
        }
    }
}

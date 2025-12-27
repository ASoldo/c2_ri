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

#[derive(Debug, Clone, PartialEq)]
pub struct OperationsState {
    pub show_flights: bool,
    pub show_ships: bool,
    pub show_satellites: bool,
    pub show_base: bool,
    pub show_map: bool,
    pub show_weather: bool,
    pub show_sea: bool,
    pub tile_provider: String,
    pub weather_field: String,
    pub sea_field: String,
}

impl Default for OperationsState {
    fn default() -> Self {
        Self {
            show_flights: true,
            show_ships: true,
            show_satellites: true,
            show_base: true,
            show_map: true,
            show_weather: true,
            show_sea: true,
            tile_provider: "osm".to_string(),
            weather_field: "IMERG_Precipitation_Rate".to_string(),
            sea_field: "OSCAR_Sea_Surface_Currents_Zonal".to_string(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct TileProviderConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub zoom_bias: i8,
}

const TILE_PROVIDERS: &[TileProviderConfig] = &[
    TileProviderConfig {
        id: "osm",
        name: "OSM Standard",
        min_zoom: 0,
        max_zoom: 19,
        zoom_bias: 0,
    },
    TileProviderConfig {
        id: "hot",
        name: "OSM Humanitarian",
        min_zoom: 0,
        max_zoom: 19,
        zoom_bias: 0,
    },
    TileProviderConfig {
        id: "opentopo",
        name: "OpenTopoMap",
        min_zoom: 0,
        max_zoom: 17,
        zoom_bias: 0,
    },
    TileProviderConfig {
        id: "nasa",
        name: "NASA Blue Marble",
        min_zoom: 0,
        max_zoom: 8,
        zoom_bias: 0,
    },
];

const WEATHER_FIELDS: &[&str] = &[
    "IMERG_Precipitation_Rate",
    "AIRS_Precipitation_Day",
    "MODIS_Terra_Cloud_Fraction_Day",
    "MODIS_Terra_Cloud_Top_Temp_Day",
    "MODIS_Terra_Cloud_Top_Pressure_Day",
    "MODIS_Terra_Cloud_Top_Height_Day",
    "MERRA2_2m_Air_Temperature_Monthly",
];

const SEA_FIELDS: &[&str] = &[
    "OSCAR_Sea_Surface_Currents_Zonal",
    "OSCAR_Sea_Surface_Currents_Meridional",
    "AMSRU_Ocean_Wind_Speed_Day",
    "JPL_MEaSUREs_L4_Sea_Surface_Height_Anomalies",
];

pub struct UiState {
    dock_state: DockState<DockTab>,
    globe_rect: Option<egui::Rect>,
    operations: OperationsState,
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
            operations: OperationsState::default(),
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        world: &WorldState,
        renderer: &Renderer,
        instances: &[RenderInstance],
        globe_texture_id: Option<egui::TextureId>,
        tiles_loading: Option<f32>,
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
                operations: &mut self.operations,
            };
            let style = Style::from_egui(ui.style());
            DockArea::new(&mut self.dock_state)
                .style(style)
                .show_inside(ui, &mut viewer);
        });

        self.draw_edge_compass(ctx, renderer, instances);
        self.draw_globe_overlay(ctx, tiles_loading);
    }

    pub fn globe_rect(&self) -> Option<egui::Rect> {
        self.globe_rect
    }

    pub fn operations(&self) -> &OperationsState {
        &self.operations
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
            let label = kind_label(instance.category);
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

    fn draw_globe_overlay(&self, ctx: &egui::Context, tiles_loading: Option<f32>) {
        let Some(rect) = self.globe_rect else {
            return;
        };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("globe-overlay"),
        ));
        let center = rect.center();
        let cross = 10.0;
        let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(220, 48, 48));
        painter.line_segment(
            [egui::pos2(center.x - cross, center.y), egui::pos2(center.x + cross, center.y)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(center.x, center.y - cross), egui::pos2(center.x, center.y + cross)],
            stroke,
        );

        if let Some(progress) = tiles_loading {
            let progress = progress.clamp(0.0, 1.0);
            let bar_height = 3.0;
            let bar_width = rect.width() * progress;
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.top()),
                egui::vec2(bar_width, bar_height),
            );
            painter.rect_filled(bar_rect, 0.0, egui::Color32::from_rgb(220, 48, 48));
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

fn kind_label(kind: u8) -> &'static str {
    match kind {
        crate::ecs::KIND_FLIGHT => "FLT",
        crate::ecs::KIND_SHIP => "SHP",
        crate::ecs::KIND_SATELLITE => "SAT",
        _ => "AST",
    }
}

fn provider_name(id: &str) -> String {
    TILE_PROVIDERS
        .iter()
        .find(|provider| provider.id == id)
        .map(|provider| provider.name.to_string())
        .unwrap_or_else(|| id.to_string())
}

pub fn tile_provider_config(id: &str) -> TileProviderConfig {
    TILE_PROVIDERS
        .iter()
        .find(|provider| provider.id == id)
        .copied()
        .unwrap_or(TileProviderConfig {
            id: "custom",
            name: "Custom",
            min_zoom: 0,
            max_zoom: 19,
            zoom_bias: 0,
        })
}

struct DockViewer<'a> {
    world: &'a WorldState,
    renderer: &'a Renderer,
    globe_rect: &'a mut Option<egui::Rect>,
    globe_texture_id: Option<egui::TextureId>,
    operations: &'a mut OperationsState,
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
                ui.label("Visibility");
                ui.checkbox(&mut self.operations.show_flights, "Flights");
                ui.checkbox(&mut self.operations.show_ships, "Ships");
                ui.checkbox(&mut self.operations.show_satellites, "Satellites");
                ui.add_space(8.0);
                ui.separator();
                ui.label("Layers");
                ui.checkbox(&mut self.operations.show_base, "Base texture");
                ui.checkbox(&mut self.operations.show_map, "Map tiles");
                ui.checkbox(&mut self.operations.show_sea, "Sea overlay");
                ui.checkbox(&mut self.operations.show_weather, "Weather overlay");
                ui.add_space(8.0);
                ui.separator();
                ui.label("Map layers");
                let provider_label = provider_name(&self.operations.tile_provider);
                egui::ComboBox::from_id_salt("tile-provider")
                    .selected_text(provider_label)
                    .show_ui(ui, |ui| {
                        for provider in TILE_PROVIDERS {
                            ui.selectable_value(
                                &mut self.operations.tile_provider,
                                provider.id.to_string(),
                                provider.name,
                            );
                        }
                    });
                ui.add_space(4.0);
                egui::ComboBox::from_id_salt("weather-field")
                    .selected_text(self.operations.weather_field.clone())
                    .show_ui(ui, |ui| {
                        for field in WEATHER_FIELDS {
                            ui.selectable_value(
                                &mut self.operations.weather_field,
                                (*field).to_string(),
                                *field,
                            );
                        }
                    });
                ui.add_space(4.0);
                egui::ComboBox::from_id_salt("sea-field")
                    .selected_text(self.operations.sea_field.clone())
                    .show_ui(ui, |ui| {
                        for field in SEA_FIELDS {
                            ui.selectable_value(
                                &mut self.operations.sea_field,
                                (*field).to_string(),
                                *field,
                            );
                        }
                    });
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

use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use glam::{Vec2, Vec3, Vec4};

use crate::ecs::{RenderInstance, WorldState};
use crate::renderer::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockTab {
    Globe,
    Operations,
    Entities,
    Inspector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockHost {
    Main,
    Detached(u64),
}

#[derive(Debug, Clone, Copy)]
pub struct DockDragStart {
    pub tab: DockTab,
    pub host: DockHost,
}

#[derive(Debug, Clone, Copy)]
pub struct DockDetachRequest {
    pub tab: DockTab,
    pub host: DockHost,
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
    main_dock: DockState<DockTab>,
    globe_rect: Option<egui::Rect>,
    operations: OperationsState,
    pending_detach: Vec<DockDetachRequest>,
    pending_attach: Vec<DockHost>,
    pending_drag_start: Option<DockDragStart>,
}

#[derive(Clone)]
pub struct TileBar {
    pub enabled: bool,
    pub progress: Option<f32>,
    pub color: egui::Color32,
}

#[derive(Clone, Copy, Default)]
pub struct PerfSnapshot {
    pub fps: f32,
    pub frame_ms: f32,
    pub frame_p95_ms: f32,
    pub frame_p99_ms: f32,
    pub world_ms: f32,
    pub tile_ms: f32,
    pub ui_ms: f32,
    pub render_ms: f32,
}

#[derive(Clone, Copy, Default)]
pub struct TileLayerStats {
    pub enabled: bool,
    pub zoom: u8,
    pub desired: usize,
    pub loaded: usize,
    pub pending: usize,
    pub cache_used: usize,
    pub cache_cap: usize,
    pub last_activity_ms: f32,
    pub stalled: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Diagnostics {
    pub perf: PerfSnapshot,
    pub map: TileLayerStats,
    pub weather: TileLayerStats,
    pub sea: TileLayerStats,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            main_dock: build_default_dock_state(),
            globe_rect: None,
            operations: OperationsState::default(),
            pending_detach: Vec::new(),
            pending_attach: Vec::new(),
            pending_drag_start: None,
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        world: &WorldState,
        renderer: &Renderer,
        instances: &[RenderInstance],
        globe_texture_id: Option<egui::TextureId>,
        tile_bars: &[TileBar],
        diagnostics: &Diagnostics,
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
                dock_host: DockHost::Main,
                world,
                renderer,
                globe_rect: Some(&mut self.globe_rect),
                globe_texture_id,
                operations: &mut self.operations,
                diagnostics,
                detach_requests: &mut self.pending_detach,
                drag_requests: &mut self.pending_drag_start,
            };
            let style = Style::from_egui(ui.style());
            DockArea::new(&mut self.main_dock)
                .style(style)
                .show_inside(ui, &mut viewer);
        });

        self.draw_edge_compass(ctx, renderer, instances, world.globe_radius());
        self.draw_globe_overlay(ctx, tile_bars);
    }

    pub fn globe_rect(&self) -> Option<egui::Rect> {
        self.globe_rect
    }

    pub fn operations(&self) -> &OperationsState {
        &self.operations
    }

    pub fn main_dock_mut(&mut self) -> &mut DockState<DockTab> {
        &mut self.main_dock
    }

    pub fn take_detach_requests(&mut self) -> Vec<DockDetachRequest> {
        std::mem::take(&mut self.pending_detach)
    }

    pub fn take_attach_requests(&mut self) -> Vec<DockHost> {
        std::mem::take(&mut self.pending_attach)
    }

    pub fn take_drag_start(&mut self) -> Option<DockDragStart> {
        self.pending_drag_start.take()
    }

    pub fn show_detached_panel(
        &mut self,
        ctx: &egui::Context,
        dock_host: DockHost,
        dock_state: &mut DockState<DockTab>,
        world: &WorldState,
        renderer: &Renderer,
        diagnostics: &Diagnostics,
    ) {
        egui::TopBottomPanel::top(format!(
            "detached-topbar-{}",
            dock_host_key(dock_host)
        ))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Dock Window");
                ui.add_space(6.0);
                if ui.button("Dock Back").clicked() {
                    self.pending_attach.push(dock_host);
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut viewer = DockViewer {
                dock_host,
                world,
                renderer,
                globe_rect: None,
                globe_texture_id: None,
                operations: &mut self.operations,
                diagnostics,
                detach_requests: &mut self.pending_detach,
                drag_requests: &mut self.pending_drag_start,
            };
            let style = Style::from_egui(ui.style());
            DockArea::new(dock_state)
                .style(style)
                .show_inside(ui, &mut viewer);
        });
    }

    fn draw_edge_compass(
        &self,
        ctx: &egui::Context,
        renderer: &Renderer,
        instances: &[RenderInstance],
        globe_radius: f32,
    ) {
        let Some(rect) = self.globe_rect else {
            return;
        };
        let bounds = rect.shrink(16.0);
        let view_proj = renderer.view_proj();
        let camera_pos = renderer.camera_position();
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
            let behind_camera = clip.w < 0.0;
            let behind_globe =
                is_occluded_by_globe(camera_pos, instance.position, globe_radius);
            if behind_camera {
                ndc = -ndc;
            }
            let on_screen = !behind_camera
                && !behind_globe
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

    fn draw_globe_overlay(&self, ctx: &egui::Context, tile_bars: &[TileBar]) {
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

        let bar_height = 3.0;
        let gap = 2.0;
        let mut bar_top = rect.top();
        for bar in tile_bars {
            if !bar.enabled {
                continue;
            }
            let background = egui::Rect::from_min_size(
                egui::pos2(rect.left(), bar_top),
                egui::vec2(rect.width(), bar_height),
            );
            painter.rect_filled(
                background,
                0.0,
                egui::Color32::from_white_alpha(28),
            );
            if let Some(progress) = bar.progress {
                let progress = progress.clamp(0.0, 1.0);
                let bar_width = rect.width() * progress;
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left(), bar_top),
                    egui::vec2(bar_width, bar_height),
                );
                painter.rect_filled(bar_rect, 0.0, bar.color);
            }
            bar_top += bar_height + gap;
        }
    }
}

fn build_default_dock_state() -> DockState<DockTab> {
    let mut dock_state = DockState::new(vec![DockTab::Globe]);
    let root = NodeIndex::root();
    let mut center = root;
    let [new_center, _left] = dock_state
        .main_surface_mut()
        .split_left(center, 0.28, vec![DockTab::Operations]);
    center = new_center;
    let [new_center, _right] = dock_state
        .main_surface_mut()
        .split_right(center, 0.28, vec![DockTab::Entities]);
    center = new_center;
    dock_state
        .main_surface_mut()
        .split_below(center, 0.28, vec![DockTab::Inspector]);
    dock_state
}

fn dock_host_key(host: DockHost) -> String {
    match host {
        DockHost::Main => "main".to_string(),
        DockHost::Detached(id) => format!("detached-{id}"),
    }
}

fn globe_panel(
    ui: &mut egui::Ui,
    globe_rect: &mut Option<egui::Rect>,
    globe_texture_id: Option<egui::TextureId>,
) {
    let available = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
    *globe_rect = Some(rect);
    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(6, 8, 12));
    if let Some(texture_id) = globe_texture_id {
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

fn operations_panel(ui: &mut egui::Ui, operations: &mut OperationsState) {
    ui.heading("Operations Menu");
    ui.separator();
    ui.label("Visibility");
    ui.checkbox(&mut operations.show_flights, "Flights");
    ui.checkbox(&mut operations.show_ships, "Ships");
    ui.checkbox(&mut operations.show_satellites, "Satellites");
    ui.add_space(8.0);
    ui.separator();
    ui.label("Layers");
    ui.checkbox(&mut operations.show_base, "Base texture");
    ui.checkbox(&mut operations.show_map, "Map tiles");
    ui.checkbox(&mut operations.show_sea, "Sea overlay");
    ui.checkbox(&mut operations.show_weather, "Weather overlay");
    ui.add_space(8.0);
    ui.separator();
    ui.label("Map layers");
    let provider_label = provider_name(&operations.tile_provider);
    egui::ComboBox::from_id_salt("tile-provider")
        .selected_text(provider_label)
        .show_ui(ui, |ui| {
            for provider in TILE_PROVIDERS {
                ui.selectable_value(
                    &mut operations.tile_provider,
                    provider.id.to_string(),
                    provider.name,
                );
            }
        });
    ui.add_space(4.0);
    egui::ComboBox::from_id_salt("weather-field")
        .selected_text(operations.weather_field.clone())
        .show_ui(ui, |ui| {
            for field in WEATHER_FIELDS {
                ui.selectable_value(
                    &mut operations.weather_field,
                    (*field).to_string(),
                    *field,
                );
            }
        });
    ui.add_space(4.0);
    egui::ComboBox::from_id_salt("sea-field")
        .selected_text(operations.sea_field.clone())
        .show_ui(ui, |ui| {
            for field in SEA_FIELDS {
                ui.selectable_value(
                    &mut operations.sea_field,
                    (*field).to_string(),
                    *field,
                );
            }
        });
    ui.add_space(8.0);
    ui.label("Status: connected to ECS runtime.");
}

fn entities_panel(ui: &mut egui::Ui, world: &WorldState) {
    ui.heading("Entities");
    ui.separator();
    ui.label(format!("Total entities: {}", world.entity_count()));
    ui.label("Filters and tasking controls will appear here.");
}

fn inspector_panel(ui: &mut egui::Ui, renderer: &Renderer, diagnostics: &Diagnostics) {
    ui.heading("Inspector");
    ui.separator();
    ui.label(format!(
        "Render targets: {}x{}",
        renderer.size().0,
        renderer.size().1
    ));
    let perf = diagnostics.perf;
    ui.add_space(4.0);
    ui.label(format!(
        "Frame: {:.1} ms (p95 {:.1} / p99 {:.1})  FPS {:.1}",
        perf.frame_ms, perf.frame_p95_ms, perf.frame_p99_ms, perf.fps
    ));
    ui.label(format!(
        "World: {:.1} ms  Tiles: {:.1} ms  UI: {:.1} ms  Render: {:.1} ms",
        perf.world_ms, perf.tile_ms, perf.ui_ms, perf.render_ms
    ));
    ui.add_space(8.0);
    ui.separator();
    ui.label("Tile cache");
    draw_tile_stats(ui, "Map", diagnostics.map);
    draw_tile_stats(ui, "Weather", diagnostics.weather);
    draw_tile_stats(ui, "Sea", diagnostics.sea);
    ui.label("Selection details will be shown here.");
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

fn draw_tile_stats(ui: &mut egui::Ui, label: &str, stats: TileLayerStats) {
    let status = if !stats.enabled {
        "off"
    } else if stats.stalled {
        "stall"
    } else {
        "ok"
    };
    ui.label(format!(
        "{label}: zoom {}  loaded {}/{}  pending {}  cache {}/{}  {status} {:.0} ms",
        stats.zoom,
        stats.loaded,
        stats.desired,
        stats.pending,
        stats.cache_used,
        stats.cache_cap,
        stats.last_activity_ms
    ));
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
    dock_host: DockHost,
    world: &'a WorldState,
    renderer: &'a Renderer,
    globe_rect: Option<&'a mut Option<egui::Rect>>,
    globe_texture_id: Option<egui::TextureId>,
    operations: &'a mut OperationsState,
    diagnostics: &'a Diagnostics,
    detach_requests: &'a mut Vec<DockDetachRequest>,
    drag_requests: &'a mut Option<DockDragStart>,
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
                if let Some(globe_rect) = self.globe_rect.as_mut() {
                    globe_panel(ui, *globe_rect, self.globe_texture_id);
                } else {
                    ui.label("Globe is available only in the main window.");
                }
            }
            DockTab::Operations => {
                operations_panel(ui, self.operations);
            }
            DockTab::Entities => {
                entities_panel(ui, self.world);
            }
            DockTab::Inspector => {
                inspector_panel(ui, self.renderer, self.diagnostics);
            }
        }
    }

    fn context_menu(
        &mut self,
        ui: &mut egui::Ui,
        tab: &mut Self::Tab,
        _surface: egui_dock::SurfaceIndex,
        _node: NodeIndex,
    ) {
        if matches!(tab, DockTab::Operations | DockTab::Entities | DockTab::Inspector) {
            if ui.button("Detach to window").clicked() {
                self.detach_requests.push(DockDetachRequest {
                    tab: *tab,
                    host: self.dock_host,
                });
                ui.close();
            }
        }
    }

    fn on_tab_button(&mut self, tab: &mut Self::Tab, response: &egui::Response) {
        if *tab != DockTab::Globe && response.drag_started() && self.drag_requests.is_none() {
            *self.drag_requests = Some(DockDragStart {
                tab: *tab,
                host: self.dock_host,
            });
        }
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }

    fn is_closeable(&self, _tab: &Self::Tab) -> bool {
        false
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

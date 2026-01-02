use iced::mouse;
use iced::widget::{
    button, checkbox, column, container, mouse_area, pick_list, progress_bar, row, scrollable,
    space, stack, text,
};
use iced::widget::canvas;
use iced::{alignment, Alignment, Background, Border, Color, Length, Point, Rectangle, Size, Theme};
use iced_wgpu::Renderer as UiRenderer;
use iced_winit::core::Element as IcedElement;
use winit::window::WindowId;

use crate::ecs::WorldState;
use crate::renderer::Renderer;

const OUTER_PADDING: f32 = 10.0;
const COLUMN_SPACING: f32 = 10.0;
const ROW_SPACING: f32 = 10.0;
const TOP_BAR_HEIGHT: f32 = 34.0;
const TAB_BAR_HEIGHT: f32 = 30.0;
const GLOBE_HEADER_HEIGHT: f32 = 26.0;
const PANEL_HEADER_SPACING: f32 = 6.0;
const PANEL_WIDTH: f32 = 300.0;
const INSPECTOR_HEIGHT: f32 = 220.0;
const PANEL_PADDING: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    Globe,
    Operations,
    Entities,
    Inspector,
}

impl PanelId {
    #[allow(dead_code)]
    pub const ALL: [PanelId; 4] = [
        PanelId::Globe,
        PanelId::Operations,
        PanelId::Entities,
        PanelId::Inspector,
    ];

    pub fn title(self) -> &'static str {
        match self {
            PanelId::Globe => "Globe",
            PanelId::Operations => "Operations",
            PanelId::Entities => "Entities",
            PanelId::Inspector => "Inspector",
        }
    }

    pub fn order(self) -> u8 {
        match self {
            PanelId::Globe => 0,
            PanelId::Operations => 1,
            PanelId::Entities => 2,
            PanelId::Inspector => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MainPanels {
    pub globe: bool,
    pub operations: bool,
    pub entities: bool,
    pub inspector: bool,
}

impl MainPanels {
    pub fn contains(self, panel: PanelId) -> bool {
        match panel {
            PanelId::Globe => self.globe,
            PanelId::Operations => self.operations,
            PanelId::Entities => self.entities,
            PanelId::Inspector => self.inspector,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct UiLayout {
    pub outer_padding: f32,
    pub column_spacing: f32,
    pub row_spacing: f32,
    pub top_bar_height: f32,
    pub tab_bar_height: f32,
    pub globe_header_height: f32,
    pub panel_width: f32,
    pub inspector_height: f32,
    pub panel_padding: f32,
}

impl UiLayout {
    pub fn new() -> Self {
        Self {
            outer_padding: OUTER_PADDING,
            column_spacing: COLUMN_SPACING,
            row_spacing: ROW_SPACING,
            top_bar_height: TOP_BAR_HEIGHT,
            tab_bar_height: TAB_BAR_HEIGHT,
            globe_header_height: GLOBE_HEADER_HEIGHT,
            panel_width: PANEL_WIDTH,
            inspector_height: INSPECTOR_HEIGHT,
            panel_padding: PANEL_PADDING,
        }
    }

    pub fn globe_rect(&self, window_size: Size, panels: MainPanels) -> Rectangle {
        if !panels.globe {
            return Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(0.0, 0.0));
        }
        let width = window_size.width;
        let height = window_size.height;
        let content_width = (width - 2.0 * self.outer_padding).max(0.0);
        let content_height = (height - 2.0 * self.outer_padding).max(0.0);

        let left_panel = if panels.operations {
            self.panel_width + self.row_spacing
        } else {
            0.0
        };
        let right_panel = if panels.entities {
            self.panel_width + self.row_spacing
        } else {
            0.0
        };
        let bottom_panel = if panels.inspector {
            self.inspector_height + self.column_spacing
        } else {
            0.0
        };

        let left = self.outer_padding + left_panel + self.panel_padding;
        let right = self.outer_padding + content_width - right_panel - self.panel_padding;
        let top = self.outer_padding
            + self.top_bar_height
            + self.column_spacing
            + self.panel_padding
            + self.globe_header_height
            + PANEL_HEADER_SPACING;
        let bottom = self.outer_padding + content_height - bottom_panel - self.panel_padding;
        let rect_width = (right - left).max(0.0);
        let rect_height = (bottom - top).max(0.0);
        Rectangle::new(
            iced::Point::new(left, top),
            Size::new(rect_width, rect_height),
        )
    }

    pub fn detached_globe_rect(&self, window_size: Size, has_tabs: bool) -> Rectangle {
        let width = window_size.width;
        let height = window_size.height;
        let content_width = (width - 2.0 * self.outer_padding).max(0.0);
        let content_height = (height - 2.0 * self.outer_padding).max(0.0);

        let left = self.outer_padding;
        let right = self.outer_padding + content_width;
        let tabs = if has_tabs {
            self.tab_bar_height + self.column_spacing
        } else {
            0.0
        };
        let top = self.outer_padding + self.top_bar_height + self.column_spacing + tabs;
        let bottom = self.outer_padding + content_height;
        let rect_width = (right - left).max(0.0);
        let rect_height = (bottom - top).max(0.0);
        Rectangle::new(
            iced::Point::new(left, top),
            Size::new(rect_width, rect_height),
        )
    }
}

#[derive(Debug, Clone)]
pub enum UiMessage {
    ToggleFlights(bool),
    ToggleShips(bool),
    ToggleSatellites(bool),
    ToggleBase(bool),
    ToggleMap(bool),
    ToggleWeather(bool),
    ToggleSea(bool),
    TileProviderSelected(TileProviderConfig),
    WeatherFieldSelected(&'static str),
    SeaFieldSelected(&'static str),
    StartDrag { panel: PanelId, window: WindowId },
    SelectTab { panel: PanelId, window: WindowId },
    DetachPanel { panel: PanelId, window: WindowId },
    DockBack { window: WindowId },
}

#[derive(Debug, Clone, Copy)]
pub struct DragPreview {
    pub panel: PanelId,
    pub cursor: Point,
}

#[derive(Debug, Clone, Copy)]
pub struct DropIndicator {
    pub rect: Rectangle,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileProviderConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub zoom_bias: i8,
}

impl std::fmt::Display for TileProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

const TILE_PROVIDERS: &[TileProviderConfig] = &[
    TileProviderConfig {
        id: "osm",
        name: "OSM Standard",
        url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
        min_zoom: 0,
        max_zoom: 19,
        zoom_bias: 0,
    },
    TileProviderConfig {
        id: "hot",
        name: "OSM Humanitarian",
        url: "https://a.tile.openstreetmap.fr/hot/{z}/{x}/{y}.png",
        min_zoom: 0,
        max_zoom: 19,
        zoom_bias: 0,
    },
    TileProviderConfig {
        id: "opentopo",
        name: "OpenTopoMap",
        url: "https://tile.opentopomap.org/{z}/{x}/{y}.png",
        min_zoom: 0,
        max_zoom: 17,
        zoom_bias: 0,
    },
    TileProviderConfig {
        id: "nasa",
        name: "NASA Blue Marble",
        url: "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_ShadedRelief/default/2013-12-01/GoogleMapsCompatible_Level8/{z}/{y}/{x}.jpg",
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
    operations: OperationsState,
    layout: UiLayout,
}

#[derive(Clone)]
pub struct TileBar {
    pub label: &'static str,
    pub enabled: bool,
    pub progress: Option<f32>,
    pub color: Color,
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

type UiElement<'a> = IcedElement<'a, UiMessage, Theme, UiRenderer>;

impl UiState {
    pub fn new() -> Self {
        Self {
            operations: OperationsState::default(),
            layout: UiLayout::new(),
        }
    }

    pub fn operations(&self) -> &OperationsState {
        &self.operations
    }

    pub fn update(&mut self, message: UiMessage) {
        match message {
            UiMessage::ToggleFlights(value) => self.operations.show_flights = value,
            UiMessage::ToggleShips(value) => self.operations.show_ships = value,
            UiMessage::ToggleSatellites(value) => self.operations.show_satellites = value,
            UiMessage::ToggleBase(value) => self.operations.show_base = value,
            UiMessage::ToggleMap(value) => self.operations.show_map = value,
            UiMessage::ToggleWeather(value) => self.operations.show_weather = value,
            UiMessage::ToggleSea(value) => self.operations.show_sea = value,
            UiMessage::TileProviderSelected(provider) => {
                self.operations.tile_provider = provider.id.to_string();
            }
            UiMessage::WeatherFieldSelected(field) => {
                self.operations.weather_field = field.to_string();
            }
            UiMessage::SeaFieldSelected(field) => {
                self.operations.sea_field = field.to_string();
            }
            _ => {}
        }
    }

    pub fn layout(&self) -> UiLayout {
        self.layout
    }

    pub fn globe_rect(&self, window_size: Size, panels: MainPanels) -> Rectangle {
        self.layout.globe_rect(window_size, panels)
    }

    pub fn detached_globe_rect(&self, window_size: Size, has_tabs: bool) -> Rectangle {
        self.layout.detached_globe_rect(window_size, has_tabs)
    }

    pub fn view_main<'a>(
        &'a self,
        window_id: WindowId,
        panels: MainPanels,
        world: &WorldState,
        renderer: &Renderer,
        diagnostics: &Diagnostics,
        tile_bars: &'a [TileBar],
        drop_target: bool,
        drag_preview: Option<DragPreview>,
        drop_indicator: Option<DropIndicator>,
    ) -> UiElement<'a> {
        let header = container(
            row![
                text("C2 Walaris").size(16),
                space().width(Length::Fixed(12.0)),
                text(format!("Entities: {}", world.entity_count())).size(12),
                space().width(Length::Fixed(12.0)),
                text(format!(
                    "Viewport: {}x{}",
                    renderer.size().0,
                    renderer.size().1
                ))
                .size(12)
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        )
        .height(Length::Fixed(self.layout.top_bar_height))
        .padding([4, 8])
        .style(top_bar_style);

        let operations_panel = panels
            .contains(PanelId::Operations)
            .then(|| {
                panel_card(
                    panel_header(PanelId::Operations, window_id, true),
                    operations_body(&self.operations),
                    Length::Fixed(self.layout.panel_width),
                    Length::Fill,
                    self.layout,
                )
            });

        let entities_panel = panels
            .contains(PanelId::Entities)
            .then(|| {
                panel_card(
                    panel_header(PanelId::Entities, window_id, true),
                    entities_body(world),
                    Length::Fixed(self.layout.panel_width),
                    Length::Fill,
                    self.layout,
                )
            });

        let inspector_panel = panels
            .contains(PanelId::Inspector)
            .then(|| {
                panel_card(
                    panel_header(PanelId::Inspector, window_id, true),
                    inspector_body(renderer, diagnostics, tile_bars),
                    Length::Fill,
                    Length::Fixed(self.layout.inspector_height),
                    self.layout,
                )
            });

        let globe_panel = panels.contains(PanelId::Globe).then(|| {
            globe_panel_card(
                panel_header(PanelId::Globe, window_id, true),
                globe_body(),
                Length::Fill,
                Length::Fill,
                self.layout,
            )
        });

        let mut main_row = row![].spacing(self.layout.row_spacing).height(Length::Fill);
        if let Some(panel) = operations_panel {
            main_row = main_row.push(panel);
        }
        if let Some(panel) = globe_panel {
            main_row = main_row.push(panel);
        } else {
            main_row = main_row.push(space().width(Length::Fill));
        }
        if let Some(panel) = entities_panel {
            main_row = main_row.push(panel);
        }

        let mut layout = column![header, main_row]
            .spacing(self.layout.column_spacing)
            .padding(self.layout.outer_padding)
            .height(Length::Fill)
            .width(Length::Fill);
        if let Some(panel) = inspector_panel {
            layout = layout.push(panel);
        }

        let globe_active = panels.contains(PanelId::Globe);
        let root = container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(root_style(drop_target, globe_active));
        let mut layers: Vec<UiElement> = vec![root.into()];
        if let Some(indicator) = drop_indicator {
            layers.push(drop_indicator_layer(indicator));
        }
        if let Some(preview) = drag_preview {
            layers.push(drag_preview_layer(preview));
        }
        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn view_detached<'a>(
        &'a self,
        window_id: WindowId,
        panels: &[PanelId],
        active: PanelId,
        world: &WorldState,
        renderer: &Renderer,
        diagnostics: &Diagnostics,
        tile_bars: &'a [TileBar],
        drop_target: bool,
        drag_preview: Option<DragPreview>,
        drop_indicator: Option<DropIndicator>,
    ) -> UiElement<'a> {
        let active_panel = panels
            .iter()
            .copied()
            .find(|panel| *panel == active)
            .or_else(|| panels.first().copied())
            .unwrap_or(PanelId::Operations);

        let header = container(
            row![
                text(active_panel.title()).size(14),
                space().width(Length::Fill),
                button(text("Dock Back").size(11)).on_press(UiMessage::DockBack { window: window_id })
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        )
        .height(Length::Fixed(self.layout.top_bar_height))
        .padding([4, 8])
        .style(top_bar_style);

        let tabs = (panels.len() > 1).then(|| {
            let mut tabs = row![].spacing(6).align_y(Alignment::Center);
            for panel in panels.iter().copied() {
                tabs = tabs.push(panel_tab(panel, window_id, panel == active_panel));
            }
            container(tabs)
                .height(Length::Fixed(self.layout.tab_bar_height))
                .padding([4, 8])
                .style(tab_bar_style)
        });

        let body = match active_panel {
            PanelId::Globe => globe_body(),
            PanelId::Operations => operations_body(&self.operations),
            PanelId::Entities => entities_body(world),
            PanelId::Inspector => inspector_body(renderer, diagnostics, tile_bars),
        };

        let globe_active = active_panel == PanelId::Globe;
        let panel = if globe_active {
            globe_panel_container(body, self.layout)
        } else {
            panel_body_container(body, self.layout)
        };

        let mut layout = column![header]
            .spacing(self.layout.column_spacing)
            .padding(self.layout.outer_padding)
            .height(Length::Fill)
            .width(Length::Fill);
        if let Some(tabs) = tabs {
            layout = layout.push(tabs);
        }
        layout = layout.push(panel);

        let root = container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(root_style(drop_target, globe_active));
        let mut layers: Vec<UiElement> = vec![root.into()];
        if let Some(indicator) = drop_indicator {
            layers.push(drop_indicator_layer(indicator));
        }
        if let Some(preview) = drag_preview {
            layers.push(drag_preview_layer(preview));
        }
        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn operations_body<'a>(operations: &'a OperationsState) -> UiElement<'a> {
    let selected_provider = TILE_PROVIDERS
        .iter()
        .copied()
        .find(|provider| provider.id == operations.tile_provider);
    let selected_weather = WEATHER_FIELDS
        .iter()
        .copied()
        .find(|field| *field == operations.weather_field);
    let selected_sea = SEA_FIELDS
        .iter()
        .copied()
        .find(|field| *field == operations.sea_field);

    let content = column![
        text("Operations Menu").size(16),
        text("Visibility").size(12),
        checkbox(operations.show_flights)
            .label("Flights")
            .on_toggle(UiMessage::ToggleFlights),
        checkbox(operations.show_ships)
            .label("Ships")
            .on_toggle(UiMessage::ToggleShips),
        checkbox(operations.show_satellites)
            .label("Satellites")
            .on_toggle(UiMessage::ToggleSatellites),
        text("Layers").size(12),
        checkbox(operations.show_base)
            .label("Base texture")
            .on_toggle(UiMessage::ToggleBase),
        checkbox(operations.show_map)
            .label("Map tiles")
            .on_toggle(UiMessage::ToggleMap),
        checkbox(operations.show_sea)
            .label("Sea overlay")
            .on_toggle(UiMessage::ToggleSea),
        checkbox(operations.show_weather)
            .label("Weather overlay")
            .on_toggle(UiMessage::ToggleWeather),
        text("Map layers").size(12),
        pick_list(TILE_PROVIDERS, selected_provider, UiMessage::TileProviderSelected)
            .width(Length::Fill)
            .placeholder("Tile provider"),
        pick_list(
            WEATHER_FIELDS,
            selected_weather,
            UiMessage::WeatherFieldSelected,
        )
        .width(Length::Fill)
        .placeholder("Weather field"),
        pick_list(SEA_FIELDS, selected_sea, UiMessage::SeaFieldSelected)
            .width(Length::Fill)
            .placeholder("Sea field"),
        text("Status: connected to ECS runtime.").size(11),
    ]
    .spacing(6);

    scrollable(content).into()
}

fn globe_body<'a>() -> UiElement<'a> {
    globe_surface().into()
}

fn entities_body<'a>(world: &WorldState) -> UiElement<'a> {
    let content = column![
        text("Entities").size(16),
        text(format!("Total entities: {}", world.entity_count())).size(12),
        text("Filters and tasking controls will appear here.").size(11),
    ]
    .spacing(6);

    content.into()
}

fn inspector_body<'a>(
    renderer: &Renderer,
    diagnostics: &Diagnostics,
    tile_bars: &'a [TileBar],
) -> UiElement<'a> {
    let perf = diagnostics.perf;
    let content = column![
        text("Inspector").size(16),
        text(format!(
            "Render targets: {}x{}",
            renderer.size().0,
            renderer.size().1
        ))
        .size(12),
        text(format!(
            "Frame: {:.1} ms (p95 {:.1} / p99 {:.1})  FPS {:.1}",
            perf.frame_ms, perf.frame_p95_ms, perf.frame_p99_ms, perf.fps
        ))
        .size(12),
        text(format!(
            "World: {:.1} ms  Tiles: {:.1} ms  UI: {:.1} ms  Render: {:.1} ms",
            perf.world_ms, perf.tile_ms, perf.ui_ms, perf.render_ms
        ))
        .size(12),
        text("Tile cache").size(12),
        text(tile_stats_line("Map", diagnostics.map)).size(11),
        text(tile_stats_line("Weather", diagnostics.weather)).size(11),
        text(tile_stats_line("Sea", diagnostics.sea)).size(11),
        tile_bar_column(tile_bars),
        text("Selection details will be shown here.").size(11),
    ]
    .spacing(6);

    scrollable(content).into()
}

fn panel_header(panel: PanelId, window_id: WindowId, allow_detach: bool) -> UiElement<'static> {
    let mut header = row![text(panel.title()).size(12)]
        .align_y(Alignment::Center)
        .spacing(6);
    header = header.push(space().width(Length::Fill));
    if allow_detach {
        header = header.push(
            button(text("Detach").size(11))
                .on_press(UiMessage::DetachPanel { panel, window: window_id }),
        );
    }

    let header = container(header)
        .padding([4, 8])
        .style(panel_header_style);

    mouse_area(header)
        .on_press(UiMessage::StartDrag {
            panel,
            window: window_id,
        })
        .interaction(mouse::Interaction::Grab)
        .into()
}

fn panel_card<'a>(
    header: UiElement<'a>,
    body: UiElement<'a>,
    width: Length,
    height: Length,
    layout: UiLayout,
) -> UiElement<'a> {
    container(column![header, body].spacing(PANEL_HEADER_SPACING))
        .width(width)
        .height(height)
        .padding(layout.panel_padding)
        .style(panel_style)
        .into()
}

fn globe_panel_card<'a>(
    header: UiElement<'a>,
    body: UiElement<'a>,
    width: Length,
    height: Length,
    layout: UiLayout,
) -> UiElement<'a> {
    container(column![header, body].spacing(PANEL_HEADER_SPACING))
        .width(width)
        .height(height)
        .padding(layout.panel_padding)
        .style(globe_panel_style)
        .into()
}

fn panel_body_container<'a>(body: UiElement<'a>, layout: UiLayout) -> UiElement<'a> {
    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(layout.panel_padding)
        .style(panel_style)
        .into()
}

fn globe_panel_container<'a>(body: UiElement<'a>, _layout: UiLayout) -> UiElement<'a> {
    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(globe_panel_style)
        .into()
}

fn panel_tab(panel: PanelId, window_id: WindowId, active: bool) -> UiElement<'static> {
    let label = text(panel.title()).size(11);
    let tab = container(label)
        .padding([4, 8])
        .style(tab_style(active));
    let tab_button = button(tab).on_press(UiMessage::SelectTab {
        panel,
        window: window_id,
    });

    mouse_area(tab_button)
        .on_press(UiMessage::StartDrag {
            panel,
            window: window_id,
        })
        .interaction(mouse::Interaction::Grab)
        .into()
}

fn tile_stats_line(label: &str, stats: TileLayerStats) -> String {
    let status = if !stats.enabled {
        "off"
    } else if stats.stalled {
        "stall"
    } else {
        "ok"
    };
    format!(
        "{label}: zoom {}  loaded {}/{}  pending {}  cache {}/{}  {status} {:.0} ms",
        stats.zoom,
        stats.loaded,
        stats.desired,
        stats.pending,
        stats.cache_used,
        stats.cache_cap,
        stats.last_activity_ms
    )
}

fn tile_bar_column<'a>(tile_bars: &'a [TileBar]) -> UiElement<'a> {
    if tile_bars.iter().all(|bar| !bar.enabled) {
        return space().into();
    }

    let bars = tile_bars.iter().filter(|bar| bar.enabled).fold(
        column![text("Tile progress").size(12)].spacing(4),
        |column, bar| {
            let progress = bar.progress.unwrap_or(0.0).clamp(0.0, 1.0);
            let bar_widget = progress_bar(0.0..=1.0, progress)
                .girth(Length::Fixed(6.0))
                .style(move |_: &Theme| progress_style(bar.color));
            column.push(column![text(bar.label).size(11), bar_widget].spacing(2))
        },
    );

    bars.into()
}

fn panel_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba8(18, 20, 26, 0.92))),
        text_color: Some(palette.background.weak.text),
        border: Border {
            color: Color::from_rgba8(72, 78, 92, 0.6),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn panel_header_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba8(28, 30, 38, 1.0))),
        text_color: Some(palette.background.weak.text),
        border: Border {
            color: Color::from_rgba8(66, 72, 88, 0.9),
            width: 1.0,
            radius: 3.0.into(),
        },
        ..Default::default()
    }
}

fn top_bar_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba8(14, 16, 20, 0.98))),
        text_color: Some(palette.background.weak.text),
        border: Border {
            color: Color::from_rgba8(62, 68, 84, 0.5),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn tab_bar_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba8(16, 18, 24, 0.9))),
        text_color: Some(palette.background.weak.text),
        border: Border {
            color: Color::from_rgba8(64, 70, 86, 0.6),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn tab_style(active: bool) -> impl Fn(&Theme) -> iced::widget::container::Style {
    move |_theme| iced::widget::container::Style {
        background: Some(Background::Color(if active {
            Color::from_rgba8(38, 42, 54, 1.0)
        } else {
            Color::from_rgba8(22, 24, 30, 1.0)
        })),
        border: Border {
            color: if active {
                Color::from_rgba8(96, 180, 240, 0.9)
            } else {
                Color::from_rgba8(58, 64, 78, 0.8)
            },
            width: if active { 1.5 } else { 1.0 },
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn root_style(drop_target: bool, globe_active: bool) -> impl Fn(&Theme) -> iced::widget::container::Style {
    move |_theme| iced::widget::container::Style {
        background: Some(Background::Color(if globe_active {
            Color::from_rgba8(8, 9, 12, 0.0)
        } else {
            Color::from_rgb8(8, 9, 12)
        })),
        border: Border {
            color: if drop_target {
                Color::from_rgba8(82, 190, 255, 0.9)
            } else {
                Color::from_rgba8(8, 9, 12, 1.0)
            },
            width: if drop_target { 2.0 } else { 0.0 },
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn globe_panel_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba8(8, 10, 16, 0.12))),
        border: Border {
            color: Color::from_rgba8(70, 76, 90, 0.75),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn progress_style(color: Color) -> iced::widget::progress_bar::Style {
    iced::widget::progress_bar::Style {
        background: Background::Color(Color::from_rgba8(28, 30, 38, 1.0)),
        bar: Background::Color(color),
        border: Border {
            color: Color::from_rgba8(28, 30, 38, 1.0),
            width: 0.0,
            radius: 3.0.into(),
        },
    }
}

fn globe_surface<'a>() -> UiElement<'a> {
    let overlay = canvas::Canvas::new(CompassOverlay)
        .width(Length::Fill)
        .height(Length::Fill);
    let content = stack([space().into(), overlay.into()]);
    content.into()
}

fn drop_indicator_layer<'a>(indicator: DropIndicator) -> UiElement<'a> {
    let overlay = canvas::Canvas::new(DropIndicatorOverlay { indicator })
        .width(Length::Fill)
        .height(Length::Fill);
    overlay.into()
}

fn drag_preview_layer<'a>(preview: DragPreview) -> UiElement<'a> {
    let overlay = canvas::Canvas::new(DragPreviewOverlay { preview })
        .width(Length::Fill)
        .height(Length::Fill);
    overlay.into()
}

#[derive(Debug, Clone, Copy)]
struct CompassOverlay;

impl canvas::Program<UiMessage, Theme, UiRenderer> for CompassOverlay {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &UiRenderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<UiRenderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let size = bounds.size();
        let radius = (size.width.min(size.height) * 0.5 - 12.0).max(0.0);
        if radius <= 1.0 {
            return vec![frame.into_geometry()];
        }
        let center = frame.center();

        let ring = canvas::Path::circle(center, radius);
        frame.stroke(
            &ring,
            canvas::Stroke::default()
                .with_width(1.6)
                .with_color(Color::from_rgba8(120, 198, 255, 0.85)),
        );

        let tick_color = Color::from_rgba8(120, 198, 255, 0.95);
        let ticks = [
            (-std::f32::consts::FRAC_PI_2, 10.0),
            (0.0, 10.0),
            (std::f32::consts::FRAC_PI_2, 10.0),
            (std::f32::consts::PI, 10.0),
            (-std::f32::consts::FRAC_PI_4, 6.0),
            (std::f32::consts::FRAC_PI_4, 6.0),
            (3.0 * std::f32::consts::FRAC_PI_4, 6.0),
            (-3.0 * std::f32::consts::FRAC_PI_4, 6.0),
        ];

        for (angle, length) in ticks {
            let (sin, cos) = angle.sin_cos();
            let start = iced::Point::new(center.x + cos * radius, center.y + sin * radius);
            let end = iced::Point::new(
                center.x + cos * (radius - length),
                center.y + sin * (radius - length),
            );
            let tick = canvas::Path::line(start, end);
            frame.stroke(
                &tick,
                canvas::Stroke::default()
                    .with_width(1.4)
                    .with_color(tick_color),
            );
        }

        let labels = [
            ("N", -std::f32::consts::FRAC_PI_2),
            ("E", 0.0),
            ("S", std::f32::consts::FRAC_PI_2),
            ("W", std::f32::consts::PI),
        ];
        for (label, angle) in labels {
            let (sin, cos) = angle.sin_cos();
            let position = iced::Point::new(
                center.x + cos * (radius + 10.0),
                center.y + sin * (radius + 10.0),
            );
            frame.fill_text(canvas::Text {
                content: label.to_string(),
                position,
                color: Color::from_rgba8(230, 242, 255, 0.95),
                size: 13.0.into(),
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Center,
                ..Default::default()
            });
        }

        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone, Copy)]
struct DropIndicatorOverlay {
    indicator: DropIndicator,
}

impl canvas::Program<UiMessage, Theme, UiRenderer> for DropIndicatorOverlay {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &UiRenderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<UiRenderer>> {
        let rect = self.indicator.rect;
        if rect.width <= 1.0 || rect.height <= 1.0 {
            return vec![];
        }
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let x = rect.x.clamp(0.0, bounds.width.max(0.0));
        let y = rect.y.clamp(0.0, bounds.height.max(0.0));
        let width = rect.width.min(bounds.width - x).max(0.0);
        let height = rect.height.min(bounds.height - y).max(0.0);
        if width <= 1.0 || height <= 1.0 {
            return vec![];
        }
        let box_rect = canvas::Path::rectangle(Point::new(x, y), Size::new(width, height));
        frame.fill(&box_rect, Color::from_rgba8(76, 140, 220, 0.18));
        frame.stroke(
            &box_rect,
            canvas::Stroke::default()
                .with_width(2.0)
                .with_color(Color::from_rgba8(110, 190, 255, 0.9)),
        );
        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone, Copy)]
struct DragPreviewOverlay {
    preview: DragPreview,
}

impl canvas::Program<UiMessage, Theme, UiRenderer> for DragPreviewOverlay {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &UiRenderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<UiRenderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let label = self.preview.panel.title();
        let text_size = 12.0;
        let padding_x = 10.0;
        let padding_y = 6.0;
        let text_width = label.len() as f32 * (text_size * 0.55);
        let width = (text_width + padding_x * 2.0).max(64.0);
        let height = text_size + padding_y * 2.0;
        let mut x = self.preview.cursor.x + 12.0;
        let mut y = self.preview.cursor.y + 12.0;
        if x + width > bounds.width {
            x = (bounds.width - width - 6.0).max(6.0);
        }
        if y + height > bounds.height {
            y = (bounds.height - height - 6.0).max(6.0);
        }
        let rect = canvas::Path::rectangle(Point::new(x, y), Size::new(width, height));
        frame.fill(&rect, Color::from_rgba8(24, 28, 38, 0.92));
        frame.stroke(
            &rect,
            canvas::Stroke::default()
                .with_width(1.0)
                .with_color(Color::from_rgba8(120, 190, 255, 0.85)),
        );
        frame.fill_text(canvas::Text {
            content: label.to_string(),
            position: Point::new(x + padding_x, y + height * 0.5),
            color: Color::from_rgba8(230, 242, 255, 0.95),
            size: text_size.into(),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            ..Default::default()
        });
        vec![frame.into_geometry()]
    }
}

pub fn tile_provider_config(id: &str) -> TileProviderConfig {
    TILE_PROVIDERS
        .iter()
        .find(|provider| provider.id == id)
        .copied()
        .unwrap_or(TileProviderConfig {
            id: "custom",
            name: "Custom",
            url: "",
            min_zoom: 0,
            max_zoom: 19,
            zoom_bias: 0,
        })
}

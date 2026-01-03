use iced::mouse;
use std::path::Path;

use iced::widget::{
    button, checkbox, column, container, mouse_area, pick_list, progress_bar, row, scrollable,
    space, stack, svg, text,
};
use iced::widget::button as button_widget;
use iced::widget::canvas;
use iced::{alignment, Alignment, Background, Border, Color, Length, Point, Rectangle, Shadow, Size, Theme};
use iced::widget::svg::Handle as SvgHandle;
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

impl std::fmt::Display for PanelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.title())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSlot {
    Left,
    Center,
    Right,
    Bottom,
}

#[derive(Debug, Clone, Default)]
pub struct DockStack {
    panels: Vec<PanelId>,
    active: Option<PanelId>,
}

impl DockStack {
    pub fn new(panel: PanelId) -> Self {
        Self {
            panels: vec![panel],
            active: Some(panel),
        }
    }

    pub fn panels(&self) -> &[PanelId] {
        &self.panels
    }

    pub fn active(&self) -> Option<PanelId> {
        if let Some(active) = self.active {
            if self.panels.contains(&active) {
                return Some(active);
            }
        }
        self.panels.first().copied()
    }

    pub fn contains(&self, panel: PanelId) -> bool {
        self.panels.contains(&panel)
    }

    pub fn is_empty(&self) -> bool {
        self.panels.is_empty()
    }

    pub fn set_active(&mut self, panel: PanelId) {
        if self.panels.contains(&panel) {
            self.active = Some(panel);
        }
    }

    pub fn insert(&mut self, panel: PanelId) {
        if !self.panels.contains(&panel) {
            self.panels.push(panel);
            self.panels.sort_by_key(|panel| panel.order());
        }
        self.active = Some(panel);
    }

    pub fn remove(&mut self, panel: PanelId) {
        self.panels.retain(|entry| *entry != panel);
        if self.active == Some(panel) || self.active.is_none() {
            self.active = self.panels.first().copied();
        } else if let Some(active) = self.active {
            if !self.panels.contains(&active) {
                self.active = self.panels.first().copied();
            }
        }
    }

}

#[derive(Debug, Clone, Default)]
pub struct DockLayout {
    left: DockStack,
    center: DockStack,
    right: DockStack,
    bottom: DockStack,
}

impl DockLayout {
    pub fn main_default() -> Self {
        Self {
            left: DockStack::new(PanelId::Operations),
            center: DockStack::new(PanelId::Globe),
            right: DockStack::new(PanelId::Entities),
            bottom: DockStack::new(PanelId::Inspector),
        }
    }

    pub fn single(panel: PanelId) -> Self {
        let mut layout = Self::default();
        layout.center = DockStack::new(panel);
        layout
    }

    pub fn slot_of(&self, panel: PanelId) -> Option<DockSlot> {
        if self.left.contains(panel) {
            Some(DockSlot::Left)
        } else if self.center.contains(panel) {
            Some(DockSlot::Center)
        } else if self.right.contains(panel) {
            Some(DockSlot::Right)
        } else if self.bottom.contains(panel) {
            Some(DockSlot::Bottom)
        } else {
            None
        }
    }

    pub fn active_slot_of(&self, panel: PanelId) -> Option<DockSlot> {
        for slot in [DockSlot::Left, DockSlot::Center, DockSlot::Right, DockSlot::Bottom] {
            if self.panel_in(slot) == Some(panel) {
                return Some(slot);
            }
        }
        None
    }

    pub fn panel_in(&self, slot: DockSlot) -> Option<PanelId> {
        self.stack(slot).active()
    }

    pub fn panels_in(&self, slot: DockSlot) -> &[PanelId] {
        self.stack(slot).panels()
    }

    pub fn set_active(&mut self, slot: DockSlot, panel: PanelId) {
        self.stack_mut(slot).set_active(panel);
    }

    pub fn insert(&mut self, slot: DockSlot, panel: PanelId) {
        self.remove(panel);
        self.stack_mut(slot).insert(panel);
    }

    pub fn remove(&mut self, panel: PanelId) {
        self.left.remove(panel);
        self.center.remove(panel);
        self.right.remove(panel);
        self.bottom.remove(panel);
    }

    pub fn is_empty(&self) -> bool {
        self.left.is_empty()
            && self.center.is_empty()
            && self.right.is_empty()
            && self.bottom.is_empty()
    }

    pub fn panels(&self) -> Vec<PanelId> {
        let mut panels = Vec::new();
        for slot in [DockSlot::Left, DockSlot::Center, DockSlot::Right, DockSlot::Bottom] {
            panels.extend(self.panels_in(slot).iter().copied());
        }
        panels.sort_by_key(|panel| panel.order());
        panels.dedup();
        panels
    }

    pub fn stack(&self, slot: DockSlot) -> &DockStack {
        match slot {
            DockSlot::Left => &self.left,
            DockSlot::Center => &self.center,
            DockSlot::Right => &self.right,
            DockSlot::Bottom => &self.bottom,
        }
    }

    fn stack_mut(&mut self, slot: DockSlot) -> &mut DockStack {
        match slot {
            DockSlot::Left => &mut self.left,
            DockSlot::Center => &mut self.center,
            DockSlot::Right => &mut self.right,
            DockSlot::Bottom => &mut self.bottom,
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

    pub fn globe_rect(&self, window_size: Size, layout: &DockLayout) -> Rectangle {
        let Some(slot) = layout.active_slot_of(PanelId::Globe) else {
            return Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(0.0, 0.0));
        };
        let outer = self.slot_rect(window_size, layout, slot);
        if outer.width <= 1.0 || outer.height <= 1.0 {
            return Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(0.0, 0.0));
        }
        let left = outer.x + self.panel_padding;
        let right = outer.x + outer.width - self.panel_padding;
        let top = outer.y + self.panel_padding + self.globe_header_height + PANEL_HEADER_SPACING;
        let bottom = outer.y + outer.height - self.panel_padding;
        Rectangle::new(
            iced::Point::new(left, top),
            Size::new((right - left).max(0.0), (bottom - top).max(0.0)),
        )
    }

    pub fn slot_rect(&self, window_size: Size, layout: &DockLayout, slot: DockSlot) -> Rectangle {
        let width = window_size.width;
        let height = window_size.height;
        let content_width = (width - 2.0 * self.outer_padding).max(0.0);
        let content_height = (height - 2.0 * self.outer_padding).max(0.0);
        if content_width <= 1.0 || content_height <= 1.0 {
            return Rectangle::new(Point::new(0.0, 0.0), Size::new(0.0, 0.0));
        }
        let header_height = self.top_bar_height;
        let bottom_present = !layout.stack(DockSlot::Bottom).is_empty();
        let bottom_height = if bottom_present {
            self.inspector_height
        } else {
            0.0
        };
        let top_spacing = self.column_spacing;
        let bottom_spacing = if bottom_present {
            self.column_spacing
        } else {
            0.0
        };
        let row_height =
            (content_height - header_height - top_spacing - bottom_height - bottom_spacing).max(0.0);
        let row_y = self.outer_padding + header_height + top_spacing;
        let row_x = self.outer_padding;

        let left_present = !layout.stack(DockSlot::Left).is_empty();
        let right_present = !layout.stack(DockSlot::Right).is_empty();
        let left_width = if left_present { self.panel_width } else { 0.0 };
        let right_width = if right_present { self.panel_width } else { 0.0 };
        let left_gap = if left_present { self.row_spacing } else { 0.0 };
        let right_gap = if right_present { self.row_spacing } else { 0.0 };
        let center_width =
            (content_width - left_width - right_width - left_gap - right_gap).max(0.0);
        let center_x = row_x + left_width + left_gap;
        let right_x = center_x + center_width + right_gap;

        match slot {
            DockSlot::Left => Rectangle::new(
                Point::new(row_x, row_y),
                Size::new(left_width, row_height),
            ),
            DockSlot::Center => Rectangle::new(
                Point::new(center_x, row_y),
                Size::new(center_width, row_height),
            ),
            DockSlot::Right => Rectangle::new(
                Point::new(right_x, row_y),
                Size::new(right_width, row_height),
            ),
            DockSlot::Bottom => Rectangle::new(
                Point::new(row_x, row_y + row_height + bottom_spacing),
                Size::new(content_width, bottom_height),
            ),
        }
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
    MinimizePanel { panel: PanelId, window: WindowId },
    RestorePanel(PanelId),
    DetachPanel { panel: PanelId, window: WindowId },
    DockBack { window: WindowId },
    SwapPanel { panel: PanelId, window: WindowId },
    ToggleMoveMenu { panel: PanelId, window: WindowId },
    MovePanel {
        panel: PanelId,
        window: WindowId,
        target: WindowId,
    },
    MoveTargetHovered { target: Option<WindowId> },
}

#[derive(Clone, Debug)]
pub struct WindowOption {
    pub id: WindowId,
    pub label: String,
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
    icons: UiIcons,
}

struct UiIcons {
    detach: SvgHandle,
    dock: SvgHandle,
    swap: SvgHandle,
    r#move: SvgHandle,
    minimize: SvgHandle,
}

impl UiIcons {
    fn new() -> Self {
        Self {
            detach: icon_handle("detach.svg"),
            dock: icon_handle("dock.svg"),
            swap: icon_handle("swap.svg"),
            r#move: icon_handle("move.svg"),
            minimize: icon_handle("minimize.svg"),
        }
    }
}

fn icon_handle(file: &str) -> SvgHandle {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("icons")
        .join(file);
    SvgHandle::from_path(path)
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
            icons: UiIcons::new(),
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

    pub fn globe_rect(&self, window_size: Size, layout: &DockLayout) -> Rectangle {
        self.layout.globe_rect(window_size, layout)
    }

    pub fn view_main<'a>(
        &'a self,
        window_id: WindowId,
        dock_layout: &DockLayout,
        world: &WorldState,
        renderer: &Renderer,
        diagnostics: &Diagnostics,
        tile_bars: &'a [TileBar],
        drop_target: bool,
        drag_preview: Option<DragPreview>,
        drop_indicator: Option<DropIndicator>,
        hidden_panels: &'a [PanelId],
        window_options: &'a [WindowOption],
        swap_selection: Option<(PanelId, WindowId)>,
        move_menu: Option<(PanelId, WindowId)>,
        move_hover_target: Option<WindowId>,
    ) -> UiElement<'a> {
        let panel_picker =
            pick_list(hidden_panels, Option::<PanelId>::None, UiMessage::RestorePanel)
            .placeholder("Panels")
            .width(Length::Fixed(160.0));
        let header = container(
            row![text("C2 Walaris").size(16), space().width(Length::Fill), panel_picker]
                .align_y(Alignment::Center)
                .spacing(8),
        )
        .height(Length::Fixed(self.layout.top_bar_height))
        .padding([4, 8])
        .style(top_bar_style);

        let left_panel = (!dock_layout.stack(DockSlot::Left).is_empty()).then(|| {
            panel_stack_card_for(
                dock_layout.stack(DockSlot::Left),
                window_id,
                &self.operations,
                world,
                renderer,
                diagnostics,
                tile_bars,
                Length::Fixed(self.layout.panel_width),
                Length::Fill,
                true,
                swap_selection,
                move_menu,
                window_options,
                move_hover_target,
                &self.icons,
                self.layout,
            )
        });
        let center_panel = (!dock_layout.stack(DockSlot::Center).is_empty()).then(|| {
            panel_stack_card_for(
                dock_layout.stack(DockSlot::Center),
                window_id,
                &self.operations,
                world,
                renderer,
                diagnostics,
                tile_bars,
                Length::Fill,
                Length::Fill,
                true,
                swap_selection,
                move_menu,
                window_options,
                move_hover_target,
                &self.icons,
                self.layout,
            )
        });
        let right_panel = (!dock_layout.stack(DockSlot::Right).is_empty()).then(|| {
            panel_stack_card_for(
                dock_layout.stack(DockSlot::Right),
                window_id,
                &self.operations,
                world,
                renderer,
                diagnostics,
                tile_bars,
                Length::Fixed(self.layout.panel_width),
                Length::Fill,
                true,
                swap_selection,
                move_menu,
                window_options,
                move_hover_target,
                &self.icons,
                self.layout,
            )
        });
        let bottom_panel = (!dock_layout.stack(DockSlot::Bottom).is_empty()).then(|| {
            panel_stack_card_for(
                dock_layout.stack(DockSlot::Bottom),
                window_id,
                &self.operations,
                world,
                renderer,
                diagnostics,
                tile_bars,
                Length::Fill,
                Length::Fixed(self.layout.inspector_height),
                true,
                swap_selection,
                move_menu,
                window_options,
                move_hover_target,
                &self.icons,
                self.layout,
            )
        });

        let mut main_row = row![].spacing(self.layout.row_spacing).height(Length::Fill);
        if let Some(panel) = left_panel {
            main_row = main_row.push(panel);
        }
        if let Some(panel) = center_panel {
            main_row = main_row.push(panel);
        } else {
            main_row = main_row.push(space().width(Length::Fill));
        }
        if let Some(panel) = right_panel {
            main_row = main_row.push(panel);
        }

        let mut layout = column![header, main_row]
            .spacing(self.layout.column_spacing)
            .padding(self.layout.outer_padding)
            .height(Length::Fill)
            .width(Length::Fill);
        if let Some(panel) = bottom_panel {
            layout = layout.push(panel);
        }

        let globe_active = dock_layout.active_slot_of(PanelId::Globe).is_some();
        let move_target = move_hover_target == Some(window_id);
        let root = container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(root_style(drop_target, globe_active, move_target));
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

    pub fn view_detached_docked<'a>(
        &'a self,
        window_id: WindowId,
        dock_layout: &DockLayout,
        world: &WorldState,
        renderer: &Renderer,
        diagnostics: &Diagnostics,
        tile_bars: &'a [TileBar],
        drop_target: bool,
        drag_preview: Option<DragPreview>,
        drop_indicator: Option<DropIndicator>,
        window_options: &'a [WindowOption],
        swap_selection: Option<(PanelId, WindowId)>,
        move_menu: Option<(PanelId, WindowId)>,
        move_hover_target: Option<WindowId>,
    ) -> UiElement<'a> {
        let header = container(
            row![
                text("Docked Panels").size(14),
                space().width(Length::Fill),
                icon_button(self.icons.dock.clone(), UiMessage::DockBack { window: window_id })
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        )
        .height(Length::Fixed(self.layout.top_bar_height))
        .padding([4, 8])
        .style(top_bar_style);

        let left_panel = (!dock_layout.stack(DockSlot::Left).is_empty()).then(|| {
            panel_stack_card_for(
                dock_layout.stack(DockSlot::Left),
                window_id,
                &self.operations,
                world,
                renderer,
                diagnostics,
                tile_bars,
                Length::Fixed(self.layout.panel_width),
                Length::Fill,
                true,
                swap_selection,
                move_menu,
                window_options,
                move_hover_target,
                &self.icons,
                self.layout,
            )
        });
        let center_panel = (!dock_layout.stack(DockSlot::Center).is_empty()).then(|| {
            panel_stack_card_for(
                dock_layout.stack(DockSlot::Center),
                window_id,
                &self.operations,
                world,
                renderer,
                diagnostics,
                tile_bars,
                Length::Fill,
                Length::Fill,
                true,
                swap_selection,
                move_menu,
                window_options,
                move_hover_target,
                &self.icons,
                self.layout,
            )
        });
        let right_panel = (!dock_layout.stack(DockSlot::Right).is_empty()).then(|| {
            panel_stack_card_for(
                dock_layout.stack(DockSlot::Right),
                window_id,
                &self.operations,
                world,
                renderer,
                diagnostics,
                tile_bars,
                Length::Fixed(self.layout.panel_width),
                Length::Fill,
                true,
                swap_selection,
                move_menu,
                window_options,
                move_hover_target,
                &self.icons,
                self.layout,
            )
        });
        let bottom_panel = (!dock_layout.stack(DockSlot::Bottom).is_empty()).then(|| {
            panel_stack_card_for(
                dock_layout.stack(DockSlot::Bottom),
                window_id,
                &self.operations,
                world,
                renderer,
                diagnostics,
                tile_bars,
                Length::Fill,
                Length::Fixed(self.layout.inspector_height),
                true,
                swap_selection,
                move_menu,
                window_options,
                move_hover_target,
                &self.icons,
                self.layout,
            )
        });

        let mut main_row = row![].spacing(self.layout.row_spacing).height(Length::Fill);
        if let Some(panel) = left_panel {
            main_row = main_row.push(panel);
        }
        if let Some(panel) = center_panel {
            main_row = main_row.push(panel);
        } else {
            main_row = main_row.push(space().width(Length::Fill));
        }
        if let Some(panel) = right_panel {
            main_row = main_row.push(panel);
        }

        let mut layout = column![header, main_row]
            .spacing(self.layout.column_spacing)
            .padding(self.layout.outer_padding)
            .height(Length::Fill)
            .width(Length::Fill);
        if let Some(panel) = bottom_panel {
            layout = layout.push(panel);
        }

        let globe_active = dock_layout.active_slot_of(PanelId::Globe).is_some();
        let move_target = move_hover_target == Some(window_id);
        let root = container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(root_style(drop_target, globe_active, move_target));
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
    world: &WorldState,
    renderer: &Renderer,
    diagnostics: &Diagnostics,
    tile_bars: &'a [TileBar],
) -> UiElement<'a> {
    let perf = diagnostics.perf;
    let content = column![
        text("Inspector").size(16),
        text(format!("Entities: {}", world.entity_count())).size(12),
        text(format!(
            "Viewport: {}x{}",
            renderer.size().0,
            renderer.size().1
        ))
        .size(12),
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

fn panel_header(
    panel: PanelId,
    window_id: WindowId,
    allow_detach: bool,
    icons: &UiIcons,
) -> UiElement<'static> {
    let drag_handle = mouse_area(container(text(panel.title()).size(12)).padding([0, 2]))
        .on_press(UiMessage::StartDrag {
            panel,
            window: window_id,
        })
        .interaction(mouse::Interaction::Grab);

    let mut header = row![drag_handle]
        .align_y(Alignment::Center)
        .spacing(6);
    header = header.push(space().width(Length::Fill));
    header = header.push(icon_button(
        icons.swap.clone(),
        UiMessage::SwapPanel {
            panel,
            window: window_id,
        },
    ));
    header = header.push(icon_button(
        icons.r#move.clone(),
        UiMessage::ToggleMoveMenu {
            panel,
            window: window_id,
        },
    ));
    header = header.push(icon_button(
        icons.minimize.clone(),
        UiMessage::MinimizePanel {
            panel,
            window: window_id,
        },
    ));
    if allow_detach {
        header = header.push(icon_button(
            icons.detach.clone(),
            UiMessage::DetachPanel {
                panel,
                window: window_id,
            },
        ));
    }

    container(header)
        .padding([4, 8])
        .style(panel_header_style)
        .into()
}

fn panel_stack_card_for<'a>(
    stack: &DockStack,
    window_id: WindowId,
    operations: &'a OperationsState,
    world: &WorldState,
    renderer: &Renderer,
    diagnostics: &Diagnostics,
    tile_bars: &'a [TileBar],
    width: Length,
    height: Length,
    allow_detach: bool,
    swap_selected: Option<(PanelId, WindowId)>,
    move_menu: Option<(PanelId, WindowId)>,
    window_options: &'a [WindowOption],
    move_hover_target: Option<WindowId>,
    icons: &UiIcons,
    layout: UiLayout,
) -> UiElement<'a> {
    let active_panel = stack.active().unwrap_or(PanelId::Operations);
    let header = panel_header(active_panel, window_id, allow_detach, icons);
    let menu = move_menu_panel(
        active_panel,
        window_id,
        move_menu,
        window_options,
        move_hover_target,
    );
    let tabs = (stack.panels().len() > 1).then(|| {
        let mut tabs = row![].spacing(6).align_y(Alignment::Center);
        for panel in stack.panels().iter().copied() {
            tabs = tabs.push(panel_tab(panel, window_id, panel == active_panel));
        }
        container(tabs)
            .height(Length::Fixed(layout.tab_bar_height))
            .padding([4, 8])
            .style(tab_bar_style)
    });
    let body = match active_panel {
        PanelId::Globe => globe_body(),
        PanelId::Operations => operations_body(operations),
        PanelId::Entities => entities_body(world),
        PanelId::Inspector => inspector_body(world, renderer, diagnostics, tile_bars),
    };
    let mut content = column![header].spacing(PANEL_HEADER_SPACING);
    if let Some(menu) = menu {
        content = content.push(menu);
    }
    if let Some(tabs) = tabs {
        content = content.push(tabs);
    }
    content = content.push(body);
    let swap_selected = swap_selected == Some((active_panel, window_id));
    if active_panel == PanelId::Globe {
        globe_panel_card(content.into(), width, height, layout, swap_selected)
    } else {
        panel_card(content.into(), width, height, layout, swap_selected)
    }
}

fn panel_card<'a>(
    content: UiElement<'a>,
    width: Length,
    height: Length,
    layout: UiLayout,
    swap_selected: bool,
) -> UiElement<'a> {
    container(content)
        .width(width)
        .height(height)
        .padding(layout.panel_padding)
        .style(panel_style(swap_selected))
        .into()
}

fn globe_panel_card<'a>(
    content: UiElement<'a>,
    width: Length,
    height: Length,
    layout: UiLayout,
    swap_selected: bool,
) -> UiElement<'a> {
    container(content)
        .width(width)
        .height(height)
        .padding(layout.panel_padding)
        .style(globe_panel_style(swap_selected))
        .into()
}

fn panel_tab(panel: PanelId, window_id: WindowId, active: bool) -> UiElement<'static> {
    let label = text(panel.title()).size(11);
    let tab = container(label)
        .padding([4, 8])
        .style(tab_style(active));
    let tab_button = button(tab)
        .style(tab_button_style)
        .on_press(UiMessage::SelectTab {
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

fn move_menu_panel<'a>(
    panel: PanelId,
    window_id: WindowId,
    move_menu: Option<(PanelId, WindowId)>,
    window_options: &'a [WindowOption],
    move_hover_target: Option<WindowId>,
) -> Option<UiElement<'a>> {
    if move_menu != Some((panel, window_id)) {
        return None;
    }
    let mut entries = column![].spacing(4);
    let mut has_targets = false;
    for option in window_options.iter().filter(|option| option.id != window_id) {
        has_targets = true;
        let active = move_hover_target == Some(option.id);
        let row = container(text(option.label.as_str()).size(11))
            .width(Length::Fill)
            .padding([4, 8])
            .style(move_menu_item_style(active));
        let row = mouse_area(row)
            .on_press(UiMessage::MovePanel {
                panel,
                window: window_id,
                target: option.id,
            })
            .on_enter(UiMessage::MoveTargetHovered {
                target: Some(option.id),
            })
            .on_exit(UiMessage::MoveTargetHovered { target: None });
        entries = entries.push(row);
    }
    if !has_targets {
        entries = entries.push(
            container(text("No other windows").size(11))
                .width(Length::Fill)
                .padding([4, 8])
                .style(move_menu_item_style(false)),
        );
    }

    Some(
        container(entries)
            .padding(6)
            .style(move_menu_style)
            .width(Length::Fill)
            .into(),
    )
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

fn panel_style(selected: bool) -> impl Fn(&Theme) -> iced::widget::container::Style {
    move |theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba8(18, 20, 26, 0.92))),
            text_color: Some(palette.background.weak.text),
            border: Border {
                color: if selected {
                    Color::from_rgba8(224, 179, 88, 0.9)
                } else {
                    Color::from_rgba8(72, 78, 92, 0.6)
                },
                width: if selected { 2.0 } else { 1.0 },
                radius: 4.0.into(),
            },
            ..Default::default()
        }
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

fn tab_button_style(_theme: &Theme, _status: button_widget::Status) -> button_widget::Style {
    button_widget::Style {
        background: None,
        text_color: Color::TRANSPARENT,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn icon_button(icon: SvgHandle, message: UiMessage) -> UiElement<'static> {
    let glyph = svg(icon).width(Length::Fixed(16.0)).height(Length::Fixed(16.0));
    button(glyph)
        .padding(4)
        .style(icon_button_style)
        .on_press(message)
        .into()
}

fn icon_button_style(_theme: &Theme, status: button_widget::Status) -> button_widget::Style {
    let (background, border_color) = match status {
        button_widget::Status::Active => (
            Color::from_rgba8(18, 20, 28, 0.0),
            Color::from_rgba8(56, 62, 78, 0.6),
        ),
        button_widget::Status::Hovered => (
            Color::from_rgba8(30, 34, 46, 0.9),
            Color::from_rgba8(120, 170, 240, 0.9),
        ),
        button_widget::Status::Pressed => (
            Color::from_rgba8(22, 26, 36, 1.0),
            Color::from_rgba8(160, 200, 255, 0.9),
        ),
        button_widget::Status::Disabled => (
            Color::from_rgba8(18, 20, 28, 0.0),
            Color::from_rgba8(46, 52, 68, 0.4),
        ),
    };
    button_widget::Style {
        background: Some(Background::Color(background)),
        text_color: Color::from_rgba8(220, 230, 244, 0.9),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

fn move_menu_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba8(12, 14, 18, 0.96))),
        text_color: Some(palette.background.weak.text),
        border: Border {
            color: Color::from_rgba8(78, 86, 102, 0.8),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn move_menu_item_style(active: bool) -> impl Fn(&Theme) -> iced::widget::container::Style {
    move |_theme| iced::widget::container::Style {
        background: Some(Background::Color(if active {
            Color::from_rgba8(36, 40, 52, 1.0)
        } else {
            Color::from_rgba8(20, 22, 30, 1.0)
        })),
        border: Border {
            color: if active {
                Color::from_rgba8(224, 179, 88, 0.9)
            } else {
                Color::from_rgba8(52, 58, 72, 0.8)
            },
            width: if active { 1.5 } else { 1.0 },
            radius: 3.0.into(),
        },
        ..Default::default()
    }
}

fn root_style(
    drop_target: bool,
    globe_active: bool,
    move_target: bool,
) -> impl Fn(&Theme) -> iced::widget::container::Style {
    move |_theme| iced::widget::container::Style {
        background: Some(Background::Color(if globe_active {
            Color::from_rgba8(8, 9, 12, 0.0)
        } else {
            Color::from_rgb8(8, 9, 12)
        })),
        border: Border {
            color: if drop_target {
                Color::from_rgba8(82, 190, 255, 0.9)
            } else if move_target {
                Color::from_rgba8(224, 179, 88, 0.9)
            } else {
                Color::from_rgba8(8, 9, 12, 1.0)
            },
            width: if drop_target || move_target { 2.0 } else { 0.0 },
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn globe_panel_style(selected: bool) -> impl Fn(&Theme) -> iced::widget::container::Style {
    move |_theme| iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba8(8, 10, 16, 0.12))),
        border: Border {
            color: if selected {
                Color::from_rgba8(224, 179, 88, 0.9)
            } else {
                Color::from_rgba8(70, 76, 90, 0.75)
            },
            width: if selected { 2.0 } else { 1.0 },
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
        let min_side = size.width.min(size.height);
        let radius = (min_side * 0.18).min(64.0);
        if radius < 18.0 {
            return vec![frame.into_geometry()];
        }
        let margin = 16.0;
        let center = iced::Point::new(
            (size.width - radius - margin).max(radius + margin),
            (radius + margin)
                .min(size.height - radius - margin)
                .max(radius + margin),
        );

        let ring = canvas::Path::circle(center, radius);
        frame.stroke(
            &ring,
            canvas::Stroke::default()
                .with_width(1.4)
                .with_color(Color::from_rgba8(120, 198, 255, 0.85)),
        );

        let tick_color = Color::from_rgba8(120, 198, 255, 0.95);
        let ticks = [
            (-std::f32::consts::FRAC_PI_2, 8.0),
            (0.0, 8.0),
            (std::f32::consts::FRAC_PI_2, 8.0),
            (std::f32::consts::PI, 8.0),
            (-std::f32::consts::FRAC_PI_4, 5.0),
            (std::f32::consts::FRAC_PI_4, 5.0),
            (3.0 * std::f32::consts::FRAC_PI_4, 5.0),
            (-3.0 * std::f32::consts::FRAC_PI_4, 5.0),
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
                    .with_width(1.2)
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
                center.x + cos * (radius + 8.0),
                center.y + sin * (radius + 8.0),
            );
            frame.fill_text(canvas::Text {
                content: label.to_string(),
                position,
                color: Color::from_rgba8(230, 242, 255, 0.95),
                size: 11.0.into(),
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
        let max_x = (bounds.width - width - 6.0).max(6.0);
        let max_y = (bounds.height - height - 6.0).max(6.0);
        x = x.clamp(6.0, max_x);
        y = y.clamp(6.0, max_y);
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

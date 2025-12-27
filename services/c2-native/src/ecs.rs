use bevy_ecs::prelude::*;
use glam::Vec3;

const KIND_UNKNOWN: u8 = 0;
const KIND_ASSET: u8 = 1;
const KIND_UNIT: u8 = 2;
const KIND_MISSION: u8 = 3;
const KIND_INCIDENT: u8 = 4;
pub const KIND_FLIGHT: u8 = 5;
pub const KIND_SATELLITE: u8 = 6;
pub const KIND_SHIP: u8 = 7;

const DEFAULT_GLOBE_RADIUS: f32 = 120.0;

#[derive(Component, Debug, Clone, Copy)]
pub struct EntityId(pub u64);

#[derive(Component, Debug, Clone, Copy)]
pub struct EntityKind(pub u8);

#[derive(Component, Debug, Clone, Copy)]
pub struct GeoPosition {
    pub lat_deg: f32,
    pub lon_deg: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Altitude(pub f32);

#[derive(Component, Debug, Clone, Copy)]
pub struct Heading(pub f32);

#[derive(Component, Debug, Clone, Copy)]
pub struct RenderSize(pub f32);

#[derive(Component, Debug, Clone, Copy)]
pub struct RenderColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RenderColor {
    fn rgba(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Cartesian {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity {
    pub dlat_deg: f32,
    pub dlon_deg: f32,
    pub dheading_deg: f32,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct GlobeRadius(pub f32);

#[derive(Resource, Debug, Clone, Copy, Default)]
struct DeltaTime(pub f32);

#[derive(Debug, Clone, Copy)]
pub struct RenderInstance {
    pub position: Vec3,
    pub size: f32,
    pub color: [f32; 4],
    pub heading_rad: f32,
    pub icon_index: u32,
    pub category: u8,
}

pub struct WorldState {
    world: World,
    schedule: Schedule,
}

impl WorldState {
    pub fn seeded() -> Self {
        let mut world = World::new();
        world.insert_resource(GlobeRadius(DEFAULT_GLOBE_RADIUS));
        world.insert_resource(DeltaTime::default());

        let mut schedule = Schedule::default();
        schedule.add_systems((advance_motion, update_cartesian));

        let mut state = Self { world, schedule };
        state.seed_demo();
        state
    }

    pub fn update(&mut self, delta: f32) {
        if let Some(mut dt) = self.world.get_resource_mut::<DeltaTime>() {
            dt.0 = delta.max(0.0);
        }
        self.schedule.run(&mut self.world);
    }

    pub fn collect_instances(&mut self, out: &mut Vec<RenderInstance>) {
        out.clear();
        let mut query = self.world.query::<(
            &Cartesian,
            &RenderSize,
            &RenderColor,
            &Heading,
            &EntityKind,
        )>();
        for (pos, size, color, heading, kind) in query.iter(&self.world) {
            out.push(RenderInstance {
                position: Vec3::new(pos.x, pos.y, pos.z),
                size: size.0,
                color: color.rgba(),
                heading_rad: heading.0.to_radians(),
                icon_index: icon_index_for_kind(kind.0),
                category: kind.0,
            });
        }
    }

    pub fn entity_count(&self) -> usize {
        self.world.entities().len() as usize
    }

    pub fn globe_radius(&self) -> f32 {
        self.world
            .get_resource::<GlobeRadius>()
            .map(|r| r.0)
            .unwrap_or(DEFAULT_GLOBE_RADIUS)
    }

    fn seed_demo(&mut self) {
        let count = std::env::var("C2_NATIVE_SEED_COUNT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(5000);
        let mut seed = 0x1e35_7bd9u32;
        for i in 0..count {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let lat = ((seed >> 16) as f32 / 65535.0) * 170.0 - 85.0;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let lon = ((seed >> 16) as f32 / 65535.0) * 360.0 - 180.0;
            let kind = match i % 3 {
                0 => KIND_FLIGHT,
                1 => KIND_SATELLITE,
                _ => KIND_SHIP,
            };
            let color = color_for_kind(kind);
            let heading = (i as f32 * 7.0) % 360.0;
            let size = size_for_kind(kind);
            let velocity = Velocity {
                dlat_deg: 0.02 + (i as f32 % 5.0) * 0.004,
                dlon_deg: 0.04 + (i as f32 % 7.0) * 0.006,
                dheading_deg: 8.0,
            };
            self.world.spawn((
                EntityId(i as u64 + 1),
                EntityKind(kind),
                GeoPosition { lat_deg: lat, lon_deg: lon },
                Altitude(altitude_for_kind(kind)),
                Heading(heading),
                RenderSize(size),
                color,
                Cartesian::default(),
                velocity,
            ));
        }
    }
}

fn update_cartesian(
    mut query: Query<(&GeoPosition, &Altitude, &mut Cartesian)>,
    radius: Res<GlobeRadius>,
) {
    for (geo, alt, mut cart) in query.iter_mut() {
        let lat = geo.lat_deg.to_radians();
        let lon = geo.lon_deg.to_radians();
        let r = radius.0 + alt.0;
        let cos_lat = lat.cos();
        cart.x = r * cos_lat * lon.cos();
        cart.y = r * lat.sin();
        cart.z = r * cos_lat * lon.sin();
    }
}

fn advance_motion(
    mut query: Query<(&mut GeoPosition, &mut Heading, &Velocity)>,
    delta: Res<DeltaTime>,
) {
    let dt = delta.0;
    if dt <= 0.0 {
        return;
    }
    for (mut geo, mut heading, velocity) in query.iter_mut() {
        geo.lat_deg = clamp_lat(geo.lat_deg + velocity.dlat_deg * dt);
        geo.lon_deg = wrap_lon(geo.lon_deg + velocity.dlon_deg * dt);
        heading.0 = (heading.0 + velocity.dheading_deg * dt) % 360.0;
    }
}

fn clamp_lat(lat: f32) -> f32 {
    lat.max(-85.0).min(85.0)
}

fn wrap_lon(lon: f32) -> f32 {
    let mut value = lon;
    while value > 180.0 {
        value -= 360.0;
    }
    while value < -180.0 {
        value += 360.0;
    }
    value
}

fn color_for_kind(kind: u8) -> RenderColor {
    match kind {
        KIND_FLIGHT => RenderColor { r: 0x38, g: 0xbd, b: 0xf8, a: 0xff },
        KIND_SATELLITE => RenderColor { r: 0xa3, g: 0xe6, b: 0x35, a: 0xff },
        KIND_SHIP => RenderColor { r: 0x22, g: 0xd3, b: 0xee, a: 0xff },
        KIND_ASSET => RenderColor { r: 0xf9, g: 0xcc, b: 0x15, a: 0xff },
        _ => RenderColor { r: 0x94, g: 0xa3, b: 0xb8, a: 0xff },
    }
}

fn size_for_kind(kind: u8) -> f32 {
    match kind {
        KIND_SHIP => 4.5,
        KIND_SATELLITE => 3.2,
        KIND_FLIGHT => 2.6,
        _ => 2.4,
    }
}

fn altitude_for_kind(kind: u8) -> f32 {
    match kind {
        KIND_SHIP => 0.6,
        KIND_SATELLITE => 28.0,
        KIND_FLIGHT => 10.0,
        _ => 0.8,
    }
}

fn icon_index_for_kind(kind: u8) -> u32 {
    match kind {
        KIND_FLIGHT => 0,
        KIND_SHIP => 1,
        KIND_SATELLITE => 2,
        _ => 0,
    }
}

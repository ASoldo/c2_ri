use bevy_ecs::prelude::*;
use bevy_ecs::schedule::Schedule;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Component, Debug, Clone, Copy)]
struct EntityId(u64);

#[derive(Component, Debug, Clone, Copy)]
struct EntityKind(u8);

const KIND_UNKNOWN: u8 = 0;
const KIND_ASSET: u8 = 1;
const KIND_UNIT: u8 = 2;
const KIND_MISSION: u8 = 3;
const KIND_INCIDENT: u8 = 4;
const KIND_FLIGHT: u8 = 5;
const KIND_SATELLITE: u8 = 6;
const KIND_SHIP: u8 = 7;
const KIND_MAX: usize = 8;
const DEFAULT_ALTITUDE: f32 = 0.0;
const DEFAULT_SIZE: f32 = 6.0;
const DEFAULT_HEADING: f32 = 0.0;

#[derive(Component, Debug, Clone, Copy)]
struct Altitude(f32);

#[derive(Component, Debug, Clone, Copy)]
struct Heading(f32);

#[derive(Component, Debug, Clone, Copy)]
struct RenderSize(f32);

#[derive(Component, Debug, Clone, Copy)]
struct RenderColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Default for RenderColor {
    fn default() -> Self {
        Self {
            r: 0x38,
            g: 0xbd,
            b: 0xf8,
            a: 0xff,
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
struct GeoPosition {
    lat_deg: f32,
    lon_deg: f32,
}

#[derive(Component, Debug, Clone, Copy, Default)]
struct Cartesian {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Resource, Debug, Clone, Copy)]
struct GlobeRadius {
    value: f32,
}

struct WorldState {
    world: World,
    schedule: Schedule,
    id_map: HashMap<u64, Entity>,
    render_ids: Vec<u64>,
    render_positions: Vec<f32>,
    render_colors: Vec<u8>,
    render_sizes: Vec<f32>,
    render_kinds: Vec<u8>,
    render_headings: Vec<f32>,
    ingest_ids: Vec<u64>,
    ingest_geos: Vec<f32>,
    ingest_kinds: Vec<u8>,
    ingest_alts: Vec<f32>,
    ingest_sizes: Vec<f32>,
    ingest_colors: Vec<u8>,
    ingest_headings: Vec<f32>,
    kind_ids: Vec<Vec<u64>>,
}

impl WorldState {
    fn new() -> Self {
        let world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(update_cartesian);
        let mut state = Self {
            world,
            schedule,
            id_map: HashMap::new(),
            render_ids: Vec::new(),
            render_positions: Vec::new(),
            render_colors: Vec::new(),
            render_sizes: Vec::new(),
            render_kinds: Vec::new(),
            render_headings: Vec::new(),
            ingest_ids: Vec::new(),
            ingest_geos: Vec::new(),
            ingest_kinds: Vec::new(),
            ingest_alts: Vec::new(),
            ingest_sizes: Vec::new(),
            ingest_colors: Vec::new(),
            ingest_headings: Vec::new(),
            kind_ids: (0..KIND_MAX).map(|_| Vec::new()).collect(),
        };
        state
            .world
            .insert_resource(GlobeRadius { value: 1.0 });
        state.seed_demo();
        state
    }

    fn seed_demo(&mut self) {
        let id = 1;
        let entity = self.world.spawn((
            EntityId(id),
            GeoPosition {
                lat_deg: 43.0,
                lon_deg: 16.0,
            },
            EntityKind(KIND_ASSET),
            Altitude(DEFAULT_ALTITUDE),
            Heading(DEFAULT_HEADING),
            RenderSize(DEFAULT_SIZE),
            RenderColor::default(),
            Cartesian::default(),
        ));
        self.id_map.insert(id, entity.id());
    }

    fn upsert_entity(
        &mut self,
        id: u64,
        lat_deg: f32,
        lon_deg: f32,
        kind: u8,
        altitude: f32,
        heading: f32,
        size: f32,
        color: RenderColor,
    ) {
        if let Some(entity) = self.id_map.get(&id).copied() {
            if let Some(mut geo) = self.world.get_mut::<GeoPosition>(entity) {
                geo.lat_deg = lat_deg;
                geo.lon_deg = lon_deg;
            }
            if let Some(mut kind_component) = self.world.get_mut::<EntityKind>(entity) {
                kind_component.0 = kind;
            } else {
                self.world.entity_mut(entity).insert(EntityKind(kind));
            }
            if let Some(mut altitude_component) = self.world.get_mut::<Altitude>(entity) {
                altitude_component.0 = altitude;
            } else {
                self.world.entity_mut(entity).insert(Altitude(altitude));
            }
            if let Some(mut heading_component) = self.world.get_mut::<Heading>(entity) {
                heading_component.0 = heading;
            } else {
                self.world.entity_mut(entity).insert(Heading(heading));
            }
            if let Some(mut size_component) = self.world.get_mut::<RenderSize>(entity) {
                size_component.0 = size;
            } else {
                self.world.entity_mut(entity).insert(RenderSize(size));
            }
            if let Some(mut color_component) = self.world.get_mut::<RenderColor>(entity) {
                *color_component = color;
            } else {
                self.world.entity_mut(entity).insert(color);
            }
            return;
        }
        let entity = self.world.spawn((
            EntityId(id),
            GeoPosition {
                lat_deg,
                lon_deg,
            },
            EntityKind(kind),
            Altitude(altitude),
            Heading(heading),
            RenderSize(size),
            color,
            Cartesian::default(),
        ));
        self.id_map.insert(id, entity.id());
    }

    fn remove_entity(&mut self, id: u64) {
        if let Some(entity) = self.id_map.remove(&id) {
            let _ = self.world.despawn(entity);
        }
    }

    fn tick(&mut self) {
        self.schedule.run(&mut self.world);
        self.refresh_render_buffers();
    }

    fn set_globe_radius(&mut self, radius: f32) {
        if let Some(mut value) = self.world.get_resource_mut::<GlobeRadius>() {
            value.value = radius.max(1.0);
        }
    }

    fn reserve_ingest(&mut self, count: usize) {
        if self.ingest_ids.len() < count {
            self.ingest_ids.resize(count, 0);
        }
        let geo_len = count * 2;
        if self.ingest_geos.len() < geo_len {
            self.ingest_geos.resize(geo_len, 0.0);
        }
        if self.ingest_kinds.len() < count {
            self.ingest_kinds.resize(count, KIND_UNKNOWN);
        }
        if self.ingest_alts.len() < count {
            let old_len = self.ingest_alts.len();
            self.ingest_alts.resize(count, DEFAULT_ALTITUDE);
            if old_len < count {
                for value in &mut self.ingest_alts[old_len..count] {
                    *value = DEFAULT_ALTITUDE;
                }
            }
        }
        if self.ingest_sizes.len() < count {
            let old_len = self.ingest_sizes.len();
            self.ingest_sizes.resize(count, DEFAULT_SIZE);
            if old_len < count {
                for value in &mut self.ingest_sizes[old_len..count] {
                    *value = DEFAULT_SIZE;
                }
            }
        }
        if self.ingest_headings.len() < count {
            let old_len = self.ingest_headings.len();
            self.ingest_headings.resize(count, DEFAULT_HEADING);
            if old_len < count {
                for value in &mut self.ingest_headings[old_len..count] {
                    *value = DEFAULT_HEADING;
                }
            }
        }
        let color_len = count * 4;
        if self.ingest_colors.len() < color_len {
            let old_len = self.ingest_colors.len();
            self.ingest_colors.resize(color_len, 0);
            if old_len < color_len {
                for chunk in self.ingest_colors[old_len..color_len].chunks_exact_mut(4) {
                    chunk[0] = RenderColor::default().r;
                    chunk[1] = RenderColor::default().g;
                    chunk[2] = RenderColor::default().b;
                    chunk[3] = RenderColor::default().a;
                }
            }
        }
    }

    fn ingest_commit(&mut self, count: usize) {
        let count = count.min(self.ingest_ids.len());
        let geo_len = count * 2;
        if self.ingest_geos.len() < geo_len {
            return;
        }
        if self.ingest_kinds.len() < count {
            return;
        }
        if self.ingest_alts.len() < count {
            return;
        }
        if self.ingest_sizes.len() < count {
            return;
        }
        if self.ingest_headings.len() < count {
            return;
        }
        if self.ingest_colors.len() < count * 4 {
            return;
        }
        for index in 0..count {
            let id = self.ingest_ids[index];
            let geo_index = index * 2;
            let lat = self.ingest_geos[geo_index];
            let lon = self.ingest_geos[geo_index + 1];
            let kind = self.ingest_kinds[index];
            let altitude = self.ingest_alts[index];
            let heading = self.ingest_headings[index];
            let size = self.ingest_sizes[index];
            let color_index = index * 4;
            let color = RenderColor {
                r: self.ingest_colors[color_index],
                g: self.ingest_colors[color_index + 1],
                b: self.ingest_colors[color_index + 2],
                a: self.ingest_colors[color_index + 3],
            };
            self.upsert_entity(id, lat, lon, kind, altitude, heading, size, color);
        }
    }

    fn refresh_render_buffers(&mut self) {
        self.render_ids.clear();
        self.render_positions.clear();
        self.render_colors.clear();
        self.render_sizes.clear();
        self.render_kinds.clear();
        self.render_headings.clear();
        self.render_ids.reserve(self.id_map.len());
        self.render_positions.reserve(self.id_map.len() * 3);
        self.render_colors.reserve(self.id_map.len() * 4);
        self.render_sizes.reserve(self.id_map.len());
        self.render_kinds.reserve(self.id_map.len());
        self.render_headings.reserve(self.id_map.len());
        for list in &mut self.kind_ids {
            list.clear();
        }
        let mut query = self
            .world
            .query::<(
                &EntityId,
                &Cartesian,
                Option<&EntityKind>,
                Option<&RenderColor>,
                Option<&RenderSize>,
                Option<&Heading>,
            )>();
        for (entity_id, cart, kind, color, size, heading) in query.iter(&self.world) {
            self.render_ids.push(entity_id.0);
            self.render_positions.push(cart.x);
            self.render_positions.push(cart.y);
            self.render_positions.push(cart.z);
            let render_color = color.copied().unwrap_or_default();
            self.render_colors.push(render_color.r);
            self.render_colors.push(render_color.g);
            self.render_colors.push(render_color.b);
            self.render_colors.push(render_color.a);
            let render_size = size.map(|value| value.0).unwrap_or(DEFAULT_SIZE);
            self.render_sizes.push(render_size);
            let index = kind.map(|value| value.0 as usize).unwrap_or(KIND_UNKNOWN as usize);
            if index < self.kind_ids.len() {
                self.kind_ids[index].push(entity_id.0);
            }
            let kind_value = kind.map(|value| value.0).unwrap_or(KIND_UNKNOWN);
            self.render_kinds.push(kind_value);
            let render_heading = heading.map(|value| value.0).unwrap_or(DEFAULT_HEADING);
            self.render_headings.push(render_heading);
        }
    }
}

fn update_cartesian(
    radius: Res<GlobeRadius>,
    mut query: Query<(&GeoPosition, Option<&Altitude>, &mut Cartesian)>,
) {
    update_cartesian_with_radius(radius.value, &mut query);
}

fn update_cartesian_with_radius(
    radius: f32,
    query: &mut Query<(&GeoPosition, Option<&Altitude>, &mut Cartesian)>,
) {
    for (geo, altitude, mut cart) in query.iter_mut() {
        let altitude = altitude.map(|value| value.0).unwrap_or(DEFAULT_ALTITUDE);
        let (x, y, z) = geo_to_cartesian(geo.lat_deg, geo.lon_deg, radius + altitude);
        cart.x = x;
        cart.y = y;
        cart.z = z;
    }
}

fn geo_to_cartesian(lat_deg: f32, lon_deg: f32, radius: f32) -> (f32, f32, f32) {
    let phi = (90.0 - lat_deg).to_radians();
    let theta = (lon_deg + 180.0).to_radians();
    let sin_phi = phi.sin();
    let x = radius * sin_phi * theta.cos();
    let y = radius * phi.cos();
    let z = radius * sin_phi * theta.sin();
    (x, y, z)
}

thread_local! {
    static ECS_STATE: RefCell<Option<WorldState>> = RefCell::new(None);
}

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut WorldState) -> R,
{
    ECS_STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        if state.is_none() {
            *state = Some(WorldState::new());
        }
        f(state.as_mut().expect("ECS state must be initialized"))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_init() {
    ECS_STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        if state.is_none() {
            *state = Some(WorldState::new());
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_reset() {
    ECS_STATE.with(|cell| {
        *cell.borrow_mut() = Some(WorldState::new());
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_tick() {
    with_state(|state| state.tick());
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_set_globe_radius(radius: f32) {
    with_state(|state| state.set_globe_radius(radius));
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_upsert_entity(id: u64, lat_deg: f32, lon_deg: f32) {
    with_state(|state| {
        state.upsert_entity(
            id,
            lat_deg,
            lon_deg,
            KIND_UNKNOWN,
            DEFAULT_ALTITUDE,
            DEFAULT_HEADING,
            DEFAULT_SIZE,
            RenderColor::default(),
        )
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_remove_entity(id: u64) {
    with_state(|state| state.remove_entity(id));
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_entity_count() -> usize {
    with_state(|state| state.id_map.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ids_ptr() -> *const u64 {
    with_state(|state| state.render_ids.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ids_len() -> usize {
    with_state(|state| state.render_ids.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_positions_ptr() -> *const f32 {
    with_state(|state| state.render_positions.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_positions_len() -> usize {
    with_state(|state| state.render_positions.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_colors_ptr() -> *const u8 {
    with_state(|state| state.render_colors.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_colors_len() -> usize {
    with_state(|state| state.render_colors.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_sizes_ptr() -> *const f32 {
    with_state(|state| state.render_sizes.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_sizes_len() -> usize {
    with_state(|state| state.render_sizes.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_kinds_ptr() -> *const u8 {
    with_state(|state| state.render_kinds.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_kinds_len() -> usize {
    with_state(|state| state.render_kinds.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_headings_ptr() -> *const f32 {
    with_state(|state| state.render_headings.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_headings_len() -> usize {
    with_state(|state| state.render_headings.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ingest_reserve(count: usize) {
    with_state(|state| state.reserve_ingest(count));
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ingest_ids_ptr() -> *mut u64 {
    with_state(|state| state.ingest_ids.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ingest_geos_ptr() -> *mut f32 {
    with_state(|state| state.ingest_geos.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ingest_kinds_ptr() -> *mut u8 {
    with_state(|state| state.ingest_kinds.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ingest_alts_ptr() -> *mut f32 {
    with_state(|state| state.ingest_alts.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ingest_sizes_ptr() -> *mut f32 {
    with_state(|state| state.ingest_sizes.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ingest_colors_ptr() -> *mut u8 {
    with_state(|state| state.ingest_colors.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ingest_headings_ptr() -> *mut f32 {
    with_state(|state| state.ingest_headings.as_mut_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_ingest_commit(count: usize) {
    with_state(|state| state.ingest_commit(count));
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_upsert_entity_kind(id: u64, lat_deg: f32, lon_deg: f32, kind: u32) {
    let kind = kind.min(u8::MAX as u32) as u8;
    with_state(|state| {
        state.upsert_entity(
            id,
            lat_deg,
            lon_deg,
            kind,
            DEFAULT_ALTITUDE,
            DEFAULT_HEADING,
            DEFAULT_SIZE,
            RenderColor::default(),
        )
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_upsert_entity_style(
    id: u64,
    lat_deg: f32,
    lon_deg: f32,
    kind: u32,
    altitude: f32,
    size: f32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    let kind = kind.min(u8::MAX as u32) as u8;
    let color = RenderColor { r, g, b, a };
    with_state(|state| {
        state.upsert_entity(id, lat_deg, lon_deg, kind, altitude, DEFAULT_HEADING, size, color)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_upsert_entity_style_heading(
    id: u64,
    lat_deg: f32,
    lon_deg: f32,
    kind: u32,
    altitude: f32,
    heading: f32,
    size: f32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    let kind = kind.min(u8::MAX as u32) as u8;
    let color = RenderColor { r, g, b, a };
    with_state(|state| state.upsert_entity(id, lat_deg, lon_deg, kind, altitude, heading, size, color));
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_kind_ids_ptr(kind: u32) -> *const u64 {
    with_state(|state| {
        let index = kind as usize;
        state
            .kind_ids
            .get(index)
            .map(|list| list.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_kind_ids_len(kind: u32) -> usize {
    with_state(|state| {
        let index = kind as usize;
        state.kind_ids.get(index).map(|list| list.len()).unwrap_or(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geo_to_cartesian_unit_sphere() {
        let (x, y, z) = geo_to_cartesian(0.0, 0.0, 1.0);
        assert!((x + 1.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);
        assert!(z.abs() < 1e-6);
    }
}

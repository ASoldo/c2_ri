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
    ingest_ids: Vec<u64>,
    ingest_geos: Vec<f32>,
    ingest_kinds: Vec<u8>,
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
            ingest_ids: Vec::new(),
            ingest_geos: Vec::new(),
            ingest_kinds: Vec::new(),
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
            Cartesian::default(),
        ));
        self.id_map.insert(id, entity.id());
    }

    fn upsert_entity(&mut self, id: u64, lat_deg: f32, lon_deg: f32, kind: u8) {
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
            return;
        }
        let entity = self.world.spawn((
            EntityId(id),
            GeoPosition {
                lat_deg,
                lon_deg,
            },
            EntityKind(kind),
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
        for index in 0..count {
            let id = self.ingest_ids[index];
            let geo_index = index * 2;
            let lat = self.ingest_geos[geo_index];
            let lon = self.ingest_geos[geo_index + 1];
            let kind = self.ingest_kinds[index];
            self.upsert_entity(id, lat, lon, kind);
        }
    }

    fn refresh_render_buffers(&mut self) {
        self.render_ids.clear();
        self.render_positions.clear();
        self.render_ids.reserve(self.id_map.len());
        self.render_positions.reserve(self.id_map.len() * 3);
        for list in &mut self.kind_ids {
            list.clear();
        }
        let mut query = self
            .world
            .query::<(&EntityId, &Cartesian, Option<&EntityKind>)>();
        for (entity_id, cart, kind) in query.iter(&self.world) {
            self.render_ids.push(entity_id.0);
            self.render_positions.push(cart.x);
            self.render_positions.push(cart.y);
            self.render_positions.push(cart.z);
            let index = kind.map(|value| value.0 as usize).unwrap_or(KIND_UNKNOWN as usize);
            if index < self.kind_ids.len() {
                self.kind_ids[index].push(entity_id.0);
            }
        }
    }
}

fn update_cartesian(radius: Res<GlobeRadius>, mut query: Query<(&GeoPosition, &mut Cartesian)>) {
    update_cartesian_with_radius(radius.value, &mut query);
}

fn update_cartesian_with_radius(radius: f32, query: &mut Query<(&GeoPosition, &mut Cartesian)>) {
    for (geo, mut cart) in query.iter_mut() {
        let (x, y, z) = geo_to_cartesian(geo.lat_deg, geo.lon_deg, radius);
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
    with_state(|state| state.upsert_entity(id, lat_deg, lon_deg, KIND_UNKNOWN));
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
pub extern "C" fn ecs_ingest_commit(count: usize) {
    with_state(|state| state.ingest_commit(count));
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_upsert_entity_kind(id: u64, lat_deg: f32, lon_deg: f32, kind: u32) {
    let kind = kind.min(u8::MAX as u32) as u8;
    with_state(|state| state.upsert_entity(id, lat_deg, lon_deg, kind));
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

import {
  MARKER_ALTITUDE,
  PARTICLE_SIZES,
  SHIP_BASE_ALTITUDE,
} from "./config.js";
import {
  ecsRuntime,
  ECS_KIND,
  ecsKindForType,
  ecsIdForKey,
  hashToGeo,
} from "./ecs.js";
import {
  altitudeForFlight,
  altitudeForSatellite,
  altitudeForShip,
  colorToRgba,
  colorForAsset,
  colorForUnit,
  colorForMission,
  colorForIncident,
  colorForFlight,
  colorForSatellite,
  colorForShip,
  formatFlightLabel,
  formatSatelliteLabel,
  formatShipLabel,
} from "./entity-utils.js";

const resolveGeo = (data, fallbackKey) => {
  const lat = Number.isFinite(data?.lat)
    ? data.lat
    : Number.isFinite(data?.latitude)
      ? data.latitude
      : null;
  const lon = Number.isFinite(data?.lon)
    ? data.lon
    : Number.isFinite(data?.longitude)
      ? data.longitude
      : null;
  if (Number.isFinite(lat) && Number.isFinite(lon)) {
    return { lat, lon };
  }
  return hashToGeo(String(data?.id ?? fallbackKey));
};

const particleSizeForKind = (kind) => {
  if (!kind) return PARTICLE_SIZES.default ?? 6.0;
  return PARTICLE_SIZES[kind] ?? PARTICLE_SIZES.default ?? 6.0;
};

const altitudeForKind = (kind, data) => {
  switch (kind) {
    case "flight":
      return MARKER_ALTITUDE + altitudeForFlight(data);
    case "satellite":
      return MARKER_ALTITUDE + altitudeForSatellite(data);
    case "ship":
      return SHIP_BASE_ALTITUDE + altitudeForShip();
    default:
      return MARKER_ALTITUDE;
  }
};

export class EntityStore {
  constructor() {
    this.entities = new Set();
    this.components = new Map();
    this.indexes = {
      entity: new Map(),
      flight: new Map(),
      satellite: new Map(),
      ship: new Map(),
    };
  }

  ensureEntity(entity) {
    if (entity === null || entity === undefined) return null;
    this.entities.add(entity);
    return entity;
  }

  addComponent(entity, type, data) {
    if (entity === null || entity === undefined) return;
    this.ensureEntity(entity);
    if (!this.components.has(type)) this.components.set(type, new Map());
    this.components.get(type).set(entity, data);
  }

  removeComponent(entity, type) {
    const map = this.components.get(type);
    if (!map) return;
    map.delete(entity);
  }

  getComponent(entity, type) {
    return this.components.get(type)?.get(entity);
  }

  removeEntity(entity) {
    this.entities.delete(entity);
    for (const map of this.components.values()) {
      map.delete(entity);
    }
  }
}

export const syncEntities = (payload, store) => {
  if (!payload || !store) return;
  const seen = new Set();
  const index = store.indexes.entity;
  const ingest = ecsRuntime.ready ? [] : null;

  const upsert = (key, data, color, pinLabel) => {
    const ecsId = ecsIdForKey(key);
    const entity = ecsId;
    index.set(key, entity);
    store.ensureEntity(entity);
    seen.add(entity);
    const geo = resolveGeo(data, key);
    store.addComponent(entity, "Geo", geo);
    if (ingest) {
      const kindName = key.split(":")[0];
      const kind = ecsKindForType(kindName);
      ingest.push({
        id: ecsId,
        lat: geo.lat,
        lon: geo.lon,
        kind,
        altitude: altitudeForKind(kindName, data),
        size: particleSizeForKind(kindName),
        color: colorToRgba(color),
      });
    }
    store.addComponent(entity, "Renderable", { color });
    store.addComponent(entity, "Meta", { kind: key.split(":")[0], data });
    if (pinLabel) {
      store.addComponent(entity, "Pin", { label: pinLabel });
    } else {
      store.removeComponent(entity, "Pin");
    }
  };

  (payload.assets || []).forEach((asset) => {
    const key = `asset:${asset.id}`;
    const pin = asset.status === "degraded" || asset.status === "lost" ? asset.name : null;
    upsert(key, asset, colorForAsset(asset), pin);
  });

  (payload.units || []).forEach((unit) => {
    const key = `unit:${unit.id}`;
    const label = unit.callsign || unit.display_name;
    upsert(key, unit, colorForUnit(unit), label);
  });

  (payload.missions || []).forEach((mission) => {
    const key = `mission:${mission.id}`;
    const pin = mission.status === "active" ? mission.name : null;
    upsert(key, mission, colorForMission(mission), pin);
  });

  (payload.incidents || []).forEach((incident) => {
    const key = `incident:${incident.id}`;
    const pin = incident.summary || incident.incident_type;
    upsert(key, incident, colorForIncident(incident), pin);
  });

  for (const [key, entity] of index.entries()) {
    if (!seen.has(entity)) {
      index.delete(key);
      ecsRuntime.removeEntity(entity);
      store.removeEntity(entity);
    }
  }

  if (ingest && ingest.length) {
    ecsRuntime.ingestBatch(ingest);
  }
};

export const syncFlights = (payload, store) => {
  if (!payload || !Array.isArray(payload.flights) || !store) return;
  const seen = new Set();
  const index = store.indexes.flight;
  const ingest = ecsRuntime.ready ? [] : null;

  payload.flights.forEach((flight) => {
    if (!Number.isFinite(flight.lat) || !Number.isFinite(flight.lon)) return;
    const key = flight.id || `${flight.callsign || "flight"}:${flight.lat}:${flight.lon}`;
    const ecsId = ecsIdForKey(key);
    const entity = ecsId;
    index.set(key, entity);
    store.ensureEntity(entity);
    seen.add(entity);
    store.addComponent(entity, "Geo", { lat: flight.lat, lon: flight.lon });
    const color = colorForFlight(flight);
    if (ingest) {
      ingest.push({
        id: ecsId,
        lat: flight.lat,
        lon: flight.lon,
        kind: ECS_KIND.flight,
        altitude: altitudeForKind("flight", flight),
        size: particleSizeForKind("flight"),
        color: colorToRgba(color),
      });
    }
    store.addComponent(entity, "Flight", flight);
    store.addComponent(entity, "Renderable", { color });
    store.addComponent(entity, "Meta", { kind: "flight", data: flight });
    store.addComponent(entity, "Pin", {
      label: formatFlightLabel(flight),
      icon: "/static/assets/plane.png",
      heading: flight.heading_deg,
      status: flight.on_ground ? "ground" : "airborne",
    });
  });

  for (const [key, entity] of index.entries()) {
    if (!seen.has(entity)) {
      index.delete(key);
      ecsRuntime.removeEntity(entity);
      store.removeEntity(entity);
    }
  }

  if (ingest && ingest.length) {
    ecsRuntime.ingestBatch(ingest);
  }
};

export const syncSatellites = (payload, store) => {
  if (!payload || !Array.isArray(payload.satellites) || !store) return;
  const seen = new Set();
  const index = store.indexes.satellite;
  const ingest = ecsRuntime.ready ? [] : null;

  payload.satellites.forEach((satellite) => {
    if (!Number.isFinite(satellite.lat) || !Number.isFinite(satellite.lon)) return;
    const key =
      satellite.id ||
      `${satellite.norad_id || "sat"}:${satellite.lat}:${satellite.lon}`;
    const ecsId = ecsIdForKey(key);
    const entity = ecsId;
    index.set(key, entity);
    store.ensureEntity(entity);
    seen.add(entity);
    store.addComponent(entity, "Geo", { lat: satellite.lat, lon: satellite.lon });
    const color = colorForSatellite(satellite);
    if (ingest) {
      ingest.push({
        id: ecsId,
        lat: satellite.lat,
        lon: satellite.lon,
        kind: ECS_KIND.satellite,
        altitude: altitudeForKind("satellite", satellite),
        size: particleSizeForKind("satellite"),
        color: colorToRgba(color),
      });
    }
    store.addComponent(entity, "Satellite", satellite);
    store.addComponent(entity, "Renderable", { color });
    store.addComponent(entity, "Meta", { kind: "satellite", data: satellite });
    store.addComponent(entity, "Pin", {
      label: formatSatelliteLabel(satellite),
    });
  });

  for (const [key, entity] of index.entries()) {
    if (!seen.has(entity)) {
      index.delete(key);
      ecsRuntime.removeEntity(entity);
      store.removeEntity(entity);
    }
  }

  if (ingest && ingest.length) {
    ecsRuntime.ingestBatch(ingest);
  }
};

export const syncShips = (payload, store) => {
  if (!payload || !Array.isArray(payload.ships) || !store) return;
  const seen = new Set();
  const index = store.indexes.ship;
  const ingest = ecsRuntime.ready ? [] : null;

  payload.ships.forEach((ship) => {
    if (!Number.isFinite(ship.lat) || !Number.isFinite(ship.lon)) return;
    const key = ship.id || `${ship.mmsi || "ship"}:${ship.lat}:${ship.lon}`;
    const ecsId = ecsIdForKey(key);
    const entity = ecsId;
    index.set(key, entity);
    store.ensureEntity(entity);
    seen.add(entity);
    store.addComponent(entity, "Geo", { lat: ship.lat, lon: ship.lon });
    const color = colorForShip(ship);
    if (ingest) {
      ingest.push({
        id: ecsId,
        lat: ship.lat,
        lon: ship.lon,
        kind: ECS_KIND.ship,
        altitude: altitudeForKind("ship", ship),
        size: particleSizeForKind("ship"),
        color: colorToRgba(color),
      });
    }
    store.addComponent(entity, "Ship", ship);
    store.addComponent(entity, "Renderable", { color });
    store.addComponent(entity, "Meta", { kind: "ship", data: ship });
    store.addComponent(entity, "Pin", { label: formatShipLabel(ship) });
  });

  for (const [key, entity] of index.entries()) {
    if (!seen.has(entity)) {
      index.delete(key);
      ecsRuntime.removeEntity(entity);
      store.removeEntity(entity);
    }
  }

  if (ingest && ingest.length) {
    ecsRuntime.ingestBatch(ingest);
  }
};

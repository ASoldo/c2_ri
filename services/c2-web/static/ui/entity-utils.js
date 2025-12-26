import {
  FLIGHT_CONFIG,
  MARKER_ALTITUDE,
  SATELLITE_CONFIG,
  SHIP_BASE_ALTITUDE,
  SHIP_CONFIG,
} from "./config.js";

export const colorForAsset = (asset) => {
  if (!asset) return "#38bdf8";
  switch (asset.status) {
    case "active":
      return "#22c55e";
    case "degraded":
      return "#f97316";
    case "lost":
      return "#ef4444";
    default:
      return "#38bdf8";
  }
};

export const colorForUnit = (unit) => {
  if (!unit) return "#38bdf8";
  switch (unit.type) {
    case "ground":
      return "#38bdf8";
    case "air":
      return "#22c55e";
    case "space":
      return "#a855f7";
    default:
      return "#38bdf8";
  }
};

export const colorForMission = (mission) => {
  if (!mission) return "#38bdf8";
  switch (mission.status) {
    case "active":
      return "#f59e0b";
    case "complete":
      return "#22c55e";
    case "canceled":
      return "#ef4444";
    default:
      return "#38bdf8";
  }
};

export const colorForIncident = (incident) => {
  if (!incident) return "#38bdf8";
  switch (incident.severity) {
    case "critical":
      return "#ef4444";
    case "high":
      return "#f97316";
    case "medium":
      return "#f59e0b";
    case "low":
      return "#22c55e";
    default:
      return "#38bdf8";
  }
};

export const colorForFlight = (flight) => {
  if (!flight) return "#38bdf8";
  if (flight.on_ground) return "#22c55e";
  if (flight.alert) return "#f97316";
  return "#38bdf8";
};

export const formatFlightLabel = (flight) => {
  if (!flight) return "FLIGHT";
  if (flight.callsign) return flight.callsign.trim();
  if (flight.registration) return flight.registration.trim();
  if (flight.id) return String(flight.id).split(":").pop();
  return "FLIGHT";
};

export const formatFlightDetails = (flight) => {
  if (!flight) return "";
  const parts = [];
  if (Number.isFinite(flight.altitude_m)) {
    parts.push(`${Math.round(flight.altitude_m)} m`);
  }
  if (Number.isFinite(flight.velocity_mps)) {
    parts.push(`${Math.round(flight.velocity_mps)} m/s`);
  }
  if (Number.isFinite(flight.heading_deg)) {
    parts.push(`${Math.round(flight.heading_deg)}°`);
  }
  if (flight.origin_country) {
    parts.push(flight.origin_country.trim());
  }
  return parts.join(" · ");
};

export const orbitBandForSatellite = (satellite) => {
  if (!satellite) return "unknown";
  const altitude = Number.isFinite(satellite.altitude_km)
    ? satellite.altitude_km
    : satellite.altitude_km || 0;
  if (altitude >= 35000) return "geo";
  if (altitude >= 8000) return "meo";
  if (altitude >= 2000) return "leo";
  return "unknown";
};

export const colorForSatellite = (satellite) => {
  switch (orbitBandForSatellite(satellite)) {
    case "meo":
      return "#38bdf8";
    case "geo":
      return "#a3e635";
    case "leo":
      return "#facc15";
    default:
      return "#94a3b8";
  }
};

export const altitudeForSatellite = (satellite) => {
  const altitudeKm = Number.isFinite(satellite?.altitude_km)
    ? satellite.altitude_km
    : 8;
  const scaled = altitudeKm * SATELLITE_CONFIG.altitudeScale;
  return Math.min(
    SATELLITE_CONFIG.altitudeMax,
    Math.max(SATELLITE_CONFIG.altitudeMin, scaled),
  );
};

export const formatSatelliteLabel = (satellite) => {
  if (!satellite) return "SAT";
  if (satellite.name) return satellite.name.trim();
  if (satellite.norad_id) return `NORAD ${satellite.norad_id}`;
  return "SAT";
};

export const formatSatelliteDetails = (satellite) => {
  if (!satellite) return "";
  const parts = [];
  if (Number.isFinite(satellite.altitude_km)) {
    parts.push(`${satellite.altitude_km.toFixed(1)} km`);
  }
  if (satellite.launch_date) {
    parts.push(satellite.launch_date);
  }
  if (satellite.country) {
    parts.push(satellite.country);
  }
  return parts.join(" · ");
};

export const vesselGroupForShip = (ship) => {
  if (!ship) return "unknown";
  const type = ship.type || ship.vessel_type || "";
  const lowered = String(type).toLowerCase();
  if (lowered.includes("tanker")) return "tanker";
  if (lowered.includes("passenger")) return "passenger";
  if (lowered.includes("fishing")) return "fishing";
  if (lowered.includes("cargo")) return "cargo";
  if (lowered.includes("other")) return "other";
  return "unknown";
};

export const colorForShip = (ship) => {
  switch (vesselGroupForShip(ship)) {
    case "cargo":
      return "#38bdf8";
    case "tanker":
      return "#f97316";
    case "passenger":
      return "#a3e635";
    case "fishing":
      return "#facc15";
    case "other":
      return "#22c55e";
    default:
      return "#94a3b8";
  }
};

export const altitudeForShip = () =>
  Number.isFinite(SHIP_CONFIG.altitude) ? SHIP_CONFIG.altitude : 0.12;

export const shipBaseAltitude = (renderer) => {
  const base = renderer?.markerAltitude ?? MARKER_ALTITUDE;
  return Math.max(0.25, SHIP_BASE_ALTITUDE || base * 0.2);
};

export const formatShipLabel = (ship) => {
  if (!ship) return "SHIP";
  const name = ship.name?.trim?.();
  if (name) return name;
  const callsign = ship.callsign?.trim?.();
  if (callsign) return callsign;
  if (Number.isFinite(ship.mmsi)) return `MMSI ${ship.mmsi}`;
  const id = ship.id?.split?.(":").pop?.();
  return id || "SHIP";
};

export const formatShipDetails = (ship) => {
  if (!ship) return "";
  const parts = [];
  if (Number.isFinite(ship.speed_knots)) {
    parts.push(`${ship.speed_knots.toFixed(1)} kt`);
  }
  const heading = Number.isFinite(ship.heading_deg)
    ? ship.heading_deg
    : ship.course_deg;
  if (Number.isFinite(heading)) {
    parts.push(`${Math.round(heading)}°`);
  }
  if (ship.destination) {
    parts.push(ship.destination.trim());
  }
  if (Number.isFinite(ship.draught_m)) {
    parts.push(`${ship.draught_m.toFixed(1)} m`);
  }
  return parts.join(" · ");
};

export const altitudeForFlight = (flight) => {
  const altitudeKm = Number.isFinite(flight?.altitude_m)
    ? flight.altitude_m / 1000
    : 8;
  return Math.min(8, Math.max(0.6, altitudeKm * FLIGHT_CONFIG.altitudeScale));
};

export const colorToRgba = (value, fallback = [0x38, 0xbd, 0xf8, 0xff]) => {
  if (!value || typeof value !== "string") return fallback.slice();
  const input = value.trim();
  if (!input.startsWith("#")) return fallback.slice();
  const hex = input.slice(1);
  if (hex.length === 3) {
    const r = Number.parseInt(hex[0] + hex[0], 16);
    const g = Number.parseInt(hex[1] + hex[1], 16);
    const b = Number.parseInt(hex[2] + hex[2], 16);
    if ([r, g, b].some((c) => Number.isNaN(c))) return fallback.slice();
    return [r, g, b, 0xff];
  }
  if (hex.length === 6 || hex.length === 8) {
    const r = Number.parseInt(hex.slice(0, 2), 16);
    const g = Number.parseInt(hex.slice(2, 4), 16);
    const b = Number.parseInt(hex.slice(4, 6), 16);
    const a = hex.length === 8 ? Number.parseInt(hex.slice(6, 8), 16) : 0xff;
    if ([r, g, b, a].some((c) => Number.isNaN(c))) return fallback.slice();
    return [r, g, b, a];
  }
  return fallback.slice();
};

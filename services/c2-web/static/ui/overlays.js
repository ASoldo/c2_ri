import * as THREE from "/static/vendor/three.module.js";
import { ecsRuntime, ECS_KIND } from "./ecs.js";
import { FLIGHT_CONFIG } from "./config.js";
import { forEachEntity } from "./utils.js";
import {
  altitudeForFlight,
  altitudeForSatellite,
  altitudeForShip,
  shipBaseAltitude,
  formatFlightLabel,
  formatFlightDetails,
  formatSatelliteLabel,
  formatSatelliteDetails,
  formatShipLabel,
  formatShipDetails,
  colorForFlight,
  colorForSatellite,
  colorForShip,
  orbitBandForSatellite,
  vesselGroupForShip,
} from "./entity-utils.js";
import { syncFlights, syncSatellites, syncShips } from "./store.js";

const positionForEntity = (entity, renderer, geo, altitude) => {
  if (!renderer || !geo) return null;
  if (renderer.mode === "globe") {
    const pos = ecsRuntime.positionForId(entity);
    if (pos) return pos;
  }
  return renderer.positionForGeo(geo, altitude);
};

class FlightTrailLayer {
  constructor(renderer) {
    this.renderer = renderer;
    this.group = new THREE.Group();
    this.group.renderOrder = 40;
    this.group.visible = false;
    this.tracks = new Map();
    if (this.renderer?.scene) {
      this.renderer.scene.add(this.group);
    }
  }

  setVisible(visible) {
    this.group.visible = visible;
  }

  altitudeForFlight(flight) {
    return altitudeForFlight(flight);
  }

  ensureTrack(flight) {
    let track = this.tracks.get(flight.id);
    if (track) return track;
    const material = new THREE.LineBasicMaterial({
      color: 0x38bdf8,
      transparent: true,
      opacity: 0.75,
    });
    material.depthTest = true;
    material.depthWrite = false;
    const geometry = new THREE.BufferGeometry();
    const line = new THREE.Line(geometry, material);
    line.renderOrder = 35;
    line.frustumCulled = true;
    this.group.add(line);

    const stalkMaterial = new THREE.LineBasicMaterial({
      color: 0xf43f5e,
      transparent: true,
      opacity: 0.85,
    });
    stalkMaterial.depthTest = false;
    stalkMaterial.depthWrite = false;
    const stalkGeometry = new THREE.BufferGeometry();
    const stalk = new THREE.Line(stalkGeometry, stalkMaterial);
    stalk.renderOrder = 82;
    stalk.frustumCulled = true;
    this.group.add(stalk);

    track = {
      line,
      stalk,
      points: [],
      lastSeen: performance.now(),
    };
    this.tracks.set(flight.id, track);
    return track;
  }

  updateTrackGeometry(track) {
    const points = track.points.map((point) =>
      this.renderer.positionForGeo(
        { lat: point.lat, lon: point.lon },
        this.renderer.markerAltitude + point.altitude,
      ),
    );
    const current = track.line.geometry;
    const attr = current.getAttribute("position");
    if (!attr || attr.count < points.length) {
      current.dispose();
      track.line.geometry = new THREE.BufferGeometry().setFromPoints(points);
      track.line.geometry.computeBoundingSphere();
      return;
    }
    const positions = attr.array;
    points.forEach((point, idx) => {
      positions[idx * 3] = point.x;
      positions[idx * 3 + 1] = point.y;
      positions[idx * 3 + 2] = point.z;
    });
    attr.needsUpdate = true;
    track.line.geometry.setDrawRange(0, points.length);
    track.line.geometry.computeBoundingSphere();
  }

  updateStalk(track, flight) {
    if (!flight) return;
    const ground = this.renderer.positionForGeo(
      { lat: flight.lat, lon: flight.lon },
      this.renderer.markerAltitude,
    );
    const tip = this.renderer.positionForGeo(
      { lat: flight.lat, lon: flight.lon },
      this.renderer.markerAltitude + this.altitudeForFlight(flight),
    );
    track.stalk.geometry.setFromPoints([ground, tip]);
    track.stalk.geometry.computeBoundingSphere();
  }

  ingest(flights) {
    if (!this.renderer) return;
    const now = performance.now();
    const seen = new Set();
    flights.forEach((flight) => {
      if (!Number.isFinite(flight.lat) || !Number.isFinite(flight.lon)) return;
      const track = this.ensureTrack(flight);
      const altitude = this.altitudeForFlight(flight);
      const last = track.points[track.points.length - 1];
      if (
        !last ||
        Math.abs(last.lat - flight.lat) > 0.08 ||
        Math.abs(last.lon - flight.lon) > 0.08
      ) {
        track.points.push({ lat: flight.lat, lon: flight.lon, altitude });
        if (track.points.length > FLIGHT_CONFIG.trailPoints) {
          track.points.splice(0, track.points.length - FLIGHT_CONFIG.trailPoints);
        }
        this.updateTrackGeometry(track);
      }
      this.updateStalk(track, flight);
      track.lastSeen = now;
      seen.add(flight.id);
    });

    for (const [id, track] of this.tracks.entries()) {
      if (seen.has(id)) continue;
      if (now - track.lastSeen > FLIGHT_CONFIG.trailMaxAgeMs) {
        this.group.remove(track.line);
        this.group.remove(track.stalk);
        track.line.geometry.dispose();
        track.stalk.geometry.dispose();
        this.tracks.delete(id);
      }
    }
  }
}

class FlightMeshLayer {
  constructor(renderer) {
    this.renderer = renderer;
    this.group = new THREE.Group();
    this.group.renderOrder = 45;
    this.group.visible = false;
    this.meshes = new Map();
    this.geometry = new THREE.PlaneGeometry(1, 1);
    this.texture = null;
    this.initTexture();
    if (this.renderer?.scene) {
      this.renderer.scene.add(this.group);
    }
  }

  initTexture() {
    const loader = new THREE.TextureLoader();
    this.texture = loader.load("/static/assets/plane.png");
    this.texture.colorSpace = THREE.SRGBColorSpace;
    this.texture.anisotropy =
      this.renderer?.renderer?.capabilities?.getMaxAnisotropy?.() || 1;
  }

  setVisible(visible) {
    this.group.visible = visible;
  }

  altitudeForFlight(flight) {
    return altitudeForFlight(flight);
  }

  scaleForDistance() {
    const distance = this.renderer?.camera?.position?.length?.() || this.renderer.defaultDistance;
    const ratio = this.renderer.defaultDistance
      ? distance / this.renderer.defaultDistance
      : 1;
    const base = this.renderer.globeRadius * 0.035;
    return Math.min(18, Math.max(4, base * ratio));
  }

  ensureMesh(entity) {
    let mesh = this.meshes.get(entity);
    if (mesh) return mesh;
    const material = new THREE.MeshBasicMaterial({
      map: this.texture,
      color: new THREE.Color(0xffffff),
      transparent: true,
      opacity: 0.95,
      depthTest: true,
      depthWrite: false,
      side: THREE.FrontSide,
    });
    material.alphaTest = 0.12;
    mesh = new THREE.Mesh(this.geometry, material);
    mesh.renderOrder = 60;
    this.group.add(mesh);
    this.meshes.set(entity, mesh);
    return mesh;
  }

  buildOrientation(latDeg, lonDeg, headingDeg) {
    const lat = THREE.MathUtils.degToRad(latDeg);
    const lon = THREE.MathUtils.degToRad(lonDeg);
    const normal = new THREE.Vector3(
      Math.cos(lat) * Math.cos(lon),
      Math.sin(lat),
      Math.cos(lat) * Math.sin(lon),
    ).normalize();
    const east = new THREE.Vector3(-Math.sin(lon), 0, Math.cos(lon)).normalize();
    const north = new THREE.Vector3(
      -Math.sin(lat) * Math.cos(lon),
      Math.cos(lat),
      -Math.sin(lat) * Math.sin(lon),
    ).normalize();
    const heading = THREE.MathUtils.degToRad(headingDeg);
    const forward = north
      .clone()
      .multiplyScalar(Math.cos(heading))
      .add(east.clone().multiplyScalar(Math.sin(heading)))
      .normalize();
    const right = normal.clone().cross(forward).normalize();
    const basis = new THREE.Matrix4().makeBasis(right, forward, normal);
    return { basis, normal };
  }

  sync(entities, store) {
    if (!this.renderer) return;
    const seen = new Set();
    const scale = this.scaleForDistance();
    forEachEntity(entities, (entity) => {
      const flight = store.getComponent(entity, "Flight");
      if (!flight) return;
      const mesh = this.ensureMesh(entity);
      const geo = store.getComponent(entity, "Geo");
      if (!geo) return;
      const altitude = this.altitudeForFlight(flight);
      const pos = positionForEntity(
        entity,
        this.renderer,
        geo,
        this.renderer.markerAltitude + altitude,
      );
      if (!pos) return;
      mesh.position.set(pos.x, pos.y, pos.z);
      mesh.scale.set(scale, scale, scale);
      const heading = Number.isFinite(flight.heading_deg) ? flight.heading_deg : 0;
      const { basis } = this.buildOrientation(flight.lat, flight.lon, heading);
      mesh.setRotationFromMatrix(basis);
      mesh.material.color.set(colorForFlight(flight));
      seen.add(entity);
    });

    for (const [entity, mesh] of this.meshes.entries()) {
      if (!seen.has(entity)) {
        this.group.remove(mesh);
        mesh.material.dispose();
        this.meshes.delete(entity);
      }
    }
  }
}

class FlightOverlay {
  constructor(renderer, store) {
    this.renderer = renderer;
    this.store = store;
    this.visible = false;
    this.trails = new FlightTrailLayer(renderer);
    this.planes = new FlightMeshLayer(renderer);
    this.lastSnapshot = null;
    this.trails.setVisible(false);
    this.planes.setVisible(false);
  }

  setVisible(visible) {
    this.visible = visible;
    this.trails.setVisible(this.visible && this.renderer?.mode === "globe");
    if (this.planes) {
      this.planes.setVisible(this.visible && this.renderer?.mode === "globe");
    }
  }

  ingest(snapshot) {
    if (!snapshot) return;
    this.lastSnapshot = snapshot;
    syncFlights(snapshot, this.store);
  }

  sync() {
    if (!this.visible) return;
    const flights = ecsRuntime.kindCache.get(ECS_KIND.flight) || [];
    this.trails.setVisible(this.visible && this.renderer?.mode === "globe");
    if (this.planes) {
      this.planes.setVisible(this.visible && this.renderer?.mode === "globe");
    }
    this.trails.ingest(this.lastSnapshot?.flights || []);
    if (this.planes) {
      this.planes.sync(flights, this.store);
    }
  }
}

class SatelliteMeshLayer {
  constructor(renderer) {
    this.renderer = renderer;
    this.group = new THREE.Group();
    this.group.renderOrder = 48;
    this.group.visible = false;
    this.meshes = new Map();
    this.texture = null;
    this.initTexture();
    if (this.renderer?.scene) {
      this.renderer.scene.add(this.group);
    }
  }

  initTexture() {
    const loader = new THREE.TextureLoader();
    this.texture = loader.load("/static/assets/satellite.svg");
    this.texture.colorSpace = THREE.SRGBColorSpace;
    this.texture.anisotropy =
      this.renderer?.renderer?.capabilities?.getMaxAnisotropy?.() || 1;
  }

  setVisible(visible) {
    this.group.visible = visible;
  }

  scaleForDistance() {
    const distance = this.renderer?.camera?.position?.length?.() || this.renderer.defaultDistance;
    const ratio = this.renderer.defaultDistance
      ? distance / this.renderer.defaultDistance
      : 1;
    const base = this.renderer.globeRadius * 0.022;
    return Math.min(14, Math.max(3, base * ratio));
  }

  ensureMesh(entity) {
    let mesh = this.meshes.get(entity);
    if (mesh) return mesh;
    const material = new THREE.SpriteMaterial({
      map: this.texture,
      color: new THREE.Color(0xffffff),
      transparent: true,
      opacity: 0.95,
      depthTest: true,
      depthWrite: false,
    });
    material.alphaTest = 0.12;
    mesh = new THREE.Sprite(material);
    mesh.renderOrder = 65;
    this.group.add(mesh);
    this.meshes.set(entity, mesh);
    return mesh;
  }

  sync(entities, store) {
    if (!this.renderer) return;
    const seen = new Set();
    const scale = this.scaleForDistance();
    forEachEntity(entities, (entity) => {
      const satellite = store.getComponent(entity, "Satellite");
      if (!satellite) return;
      const mesh = this.ensureMesh(entity);
      const geo = store.getComponent(entity, "Geo");
      if (!geo) return;
      const altitude = altitudeForSatellite(satellite);
      const pos = positionForEntity(
        entity,
        this.renderer,
        geo,
        this.renderer.markerAltitude + altitude,
      );
      if (!pos) return;
      mesh.position.set(pos.x, pos.y, pos.z);
      mesh.scale.set(scale, scale, scale);
      mesh.material.color.set(colorForSatellite(satellite));
      seen.add(entity);
    });

    for (const [entity, mesh] of this.meshes.entries()) {
      if (!seen.has(entity)) {
        this.group.remove(mesh);
        mesh.material.dispose();
        this.meshes.delete(entity);
      }
    }
  }
}

class SatelliteOverlay {
  constructor(renderer, store) {
    this.renderer = renderer;
    this.store = store;
    this.visible = false;
    this.meshes = new SatelliteMeshLayer(renderer);
    this.lastSnapshot = null;
    this.meshes.setVisible(false);
  }

  setVisible(visible) {
    this.visible = visible;
    if (this.meshes) {
      this.meshes.setVisible(visible && this.renderer?.mode === "globe");
    }
  }

  ingest(snapshot) {
    if (!snapshot) return;
    this.lastSnapshot = snapshot;
    syncSatellites(snapshot, this.store);
  }

  sync() {
    if (!this.visible) return;
    const satellites = ecsRuntime.kindCache.get(ECS_KIND.satellite) || [];
    if (this.meshes) {
      this.meshes.setVisible(this.visible && this.renderer?.mode === "globe");
      this.meshes.sync(satellites, this.store);
    }
  }
}

class ShipMeshLayer {
  constructor(renderer) {
    this.renderer = renderer;
    this.group = new THREE.Group();
    this.group.renderOrder = 50;
    this.group.visible = false;
    this.meshes = new Map();
    this.texture = null;
    this.initTexture();
    if (this.renderer?.scene) {
      this.renderer.scene.add(this.group);
    }
  }

  initTexture() {
    const loader = new THREE.TextureLoader();
    this.texture = loader.load("/static/assets/ship.svg");
    this.texture.colorSpace = THREE.SRGBColorSpace;
    this.texture.anisotropy =
      this.renderer?.renderer?.capabilities?.getMaxAnisotropy?.() || 1;
  }

  setVisible(visible) {
    this.group.visible = visible;
  }

  scaleForDistance() {
    const distance = this.renderer?.camera?.position?.length?.() || this.renderer.defaultDistance;
    const ratio = this.renderer.defaultDistance
      ? distance / this.renderer.defaultDistance
      : 1;
    const base = this.renderer.globeRadius * 0.026;
    return Math.min(16, Math.max(3, base * ratio));
  }

  ensureMesh(entity) {
    let mesh = this.meshes.get(entity);
    if (mesh) return mesh;
    const material = new THREE.SpriteMaterial({
      map: this.texture,
      color: new THREE.Color(0xffffff),
      transparent: true,
      opacity: 0.96,
      depthTest: true,
      depthWrite: false,
    });
    material.alphaTest = 0.12;
    mesh = new THREE.Sprite(material);
    mesh.renderOrder = 68;
    this.group.add(mesh);
    this.meshes.set(entity, mesh);
    return mesh;
  }

  sync(entities, store) {
    if (!this.renderer) return;
    const seen = new Set();
    const scale = this.scaleForDistance();
    const baseAltitude = shipBaseAltitude(this.renderer);
    forEachEntity(entities, (entity) => {
      const ship = store.getComponent(entity, "Ship");
      if (!ship) return;
      const mesh = this.ensureMesh(entity);
      const geo = store.getComponent(entity, "Geo");
      if (!geo) return;
      const pos = positionForEntity(
        entity,
        this.renderer,
        geo,
        baseAltitude + altitudeForShip(ship),
      );
      if (!pos) return;
      mesh.position.set(pos.x, pos.y, pos.z);
      mesh.scale.set(scale, scale, scale);
      mesh.material.color.set(colorForShip(ship));
      const heading = Number.isFinite(ship.heading_deg)
        ? ship.heading_deg
        : ship.course_deg;
      if (Number.isFinite(heading)) {
        mesh.material.rotation = -THREE.MathUtils.degToRad(heading);
      }
      seen.add(entity);
    });

    for (const [entity, mesh] of this.meshes.entries()) {
      if (!seen.has(entity)) {
        this.group.remove(mesh);
        mesh.material.dispose();
        this.meshes.delete(entity);
      }
    }
  }
}

class ShipOverlay {
  constructor(renderer, store) {
    this.renderer = renderer;
    this.store = store;
    this.visible = false;
    this.meshes = new ShipMeshLayer(renderer);
    this.lastSnapshot = null;
    this.meshes.setVisible(false);
  }

  setVisible(visible) {
    this.visible = visible;
    if (this.meshes) {
      this.meshes.setVisible(visible && this.renderer?.mode === "globe");
    }
  }

  ingest(snapshot) {
    if (!snapshot) return;
    this.lastSnapshot = snapshot;
    syncShips(snapshot, this.store);
  }

  sync() {
    if (!this.visible) return;
    const ships = ecsRuntime.kindCache.get(ECS_KIND.ship) || [];
    if (this.meshes) {
      this.meshes.setVisible(this.visible && this.renderer?.mode === "globe");
      this.meshes.sync(ships, this.store);
    }
  }
}

const edgeSymbolFor = (meta) => {
  if (!meta) return "?";
  switch (meta.kind) {
    case "asset":
      return "A";
    case "unit":
      return "U";
    case "mission":
      return "M";
    case "incident":
      return "I";
    case "flight":
      return "FL";
    case "satellite":
      return "SAT";
    case "ship":
      return "SH";
    default:
      return "?";
  }
};

const collapseLabel = (label) => {
  if (!label) return "";
  const trimmed = label.trim();
  if (!trimmed) return "";
  const words = trimmed.split(/\s+/).filter(Boolean);
  if (words.length > 1) {
    return words.map((word) => word[0]).join("").slice(0, 3).toUpperCase();
  }
  if (trimmed.length <= 3) return trimmed.toUpperCase();
  return trimmed.slice(0, 3).toUpperCase();
};

const BUBBLE_FONT_STACK =
  '"IBM Plex Sans", "Space Grotesk", "Manrope", sans-serif';

const BUBBLE_PIN_BASE = {
  shape: "pill",
  fontSize: 11,
  fontWeight: 600,
  paddingX: 10,
  paddingY: 6,
  textColor: "#0f172a",
  borderWidth: 1,
  shadowBlur: 14,
  uppercase: false,
  letterSpacing: 0,
};

const PIN_STYLE_DEFAULT = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(125, 211, 252, 0.9)",
  borderColor: "rgba(125, 211, 252, 0.6)",
  shadowColor: "rgba(125, 211, 252, 0.25)",
};

const PIN_STYLE_SELECTED = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(250, 204, 21, 0.95)",
  borderColor: "rgba(250, 204, 21, 0.45)",
  borderWidth: 2,
  shadowColor: "rgba(250, 204, 21, 0.4)",
  shadowBlur: 10,
  textColor: "#1e293b",
};

const PIN_STYLE_FLIGHT = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(34, 211, 238, 0.9)",
  borderColor: "rgba(34, 211, 238, 0.55)",
  shadowColor: "rgba(34, 211, 238, 0.25)",
};

const PIN_STYLE_FLIGHT_GROUND = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(148, 163, 184, 0.8)",
  borderColor: "rgba(148, 163, 184, 0.7)",
  shadowColor: "rgba(148, 163, 184, 0.3)",
};

const PIN_STYLE_SATELLITE = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(250, 204, 21, 0.9)",
  borderColor: "rgba(250, 204, 21, 0.6)",
  shadowColor: "rgba(250, 204, 21, 0.25)",
};

const PIN_STYLE_SATELLITE_MEO = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(56, 189, 248, 0.85)",
  borderColor: "rgba(56, 189, 248, 0.7)",
  shadowColor: "rgba(56, 189, 248, 0.25)",
};

const PIN_STYLE_SATELLITE_GEO = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(163, 230, 53, 0.85)",
  borderColor: "rgba(163, 230, 53, 0.7)",
  shadowColor: "rgba(163, 230, 53, 0.25)",
};

const PIN_STYLE_SATELLITE_UNKNOWN = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(148, 163, 184, 0.8)",
  borderColor: "rgba(148, 163, 184, 0.7)",
  shadowColor: "rgba(148, 163, 184, 0.3)",
};

const PIN_STYLE_SHIP = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(56, 189, 248, 0.9)",
  borderColor: "rgba(56, 189, 248, 0.6)",
  shadowColor: "rgba(56, 189, 248, 0.25)",
};

const PIN_STYLE_SHIP_TANKER = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(249, 115, 22, 0.88)",
  borderColor: "rgba(249, 115, 22, 0.7)",
  shadowColor: "rgba(249, 115, 22, 0.25)",
};

const PIN_STYLE_SHIP_PASSENGER = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(163, 230, 53, 0.88)",
  borderColor: "rgba(163, 230, 53, 0.7)",
  shadowColor: "rgba(163, 230, 53, 0.25)",
};

const PIN_STYLE_SHIP_FISHING = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(250, 204, 21, 0.88)",
  borderColor: "rgba(250, 204, 21, 0.7)",
  shadowColor: "rgba(250, 204, 21, 0.25)",
};

const PIN_STYLE_SHIP_UNKNOWN = {
  ...BUBBLE_PIN_BASE,
  background: "rgba(148, 163, 184, 0.8)",
  borderColor: "rgba(148, 163, 184, 0.7)",
  shadowColor: "rgba(148, 163, 184, 0.3)",
};

const BUBBLE_EDGE_BASE = {
  shape: "circle",
  size: 34,
  fontSize: 12,
  fontWeight: 700,
  textColor: "#0f172a",
  borderWidth: 1,
  shadowBlur: 16,
  uppercase: true,
  letterSpacing: 0.06,
  paddingX: 0,
  paddingY: 0,
};

const EDGE_STYLE_DEFAULT = {
  ...BUBBLE_EDGE_BASE,
  background: "rgba(34, 211, 238, 0.9)",
  borderColor: "rgba(34, 211, 238, 0.6)",
  shadowColor: "rgba(34, 211, 238, 0.35)",
};

const EDGE_STYLE_OCCLUDED = {
  ...BUBBLE_EDGE_BASE,
  background: "rgba(15, 23, 42, 0.9)",
  borderColor: "rgba(148, 163, 184, 0.5)",
  shadowColor: "rgba(15, 23, 42, 0.35)",
  textColor: "#e2e8f0",
};

const EDGE_STYLE_SELECTED = {
  ...BUBBLE_EDGE_BASE,
  background: "rgba(250, 204, 21, 0.95)",
  borderColor: "rgba(217, 119, 6, 0.8)",
  shadowColor: "rgba(250, 204, 21, 0.4)",
};

const EDGE_STYLE_FLIGHT = {
  ...BUBBLE_EDGE_BASE,
  background: "rgba(14, 165, 233, 0.9)",
  borderColor: "rgba(14, 165, 233, 0.7)",
  shadowColor: "rgba(14, 165, 233, 0.35)",
};

const EDGE_STYLE_SATELLITE = {
  ...BUBBLE_EDGE_BASE,
  background: "rgba(250, 204, 21, 0.9)",
  borderColor: "rgba(250, 204, 21, 0.7)",
  shadowColor: "rgba(250, 204, 21, 0.35)",
};

const EDGE_STYLE_SHIP = {
  ...BUBBLE_EDGE_BASE,
  background: "rgba(56, 189, 248, 0.9)",
  borderColor: "rgba(56, 189, 248, 0.7)",
  shadowColor: "rgba(56, 189, 248, 0.35)",
};

const EDGE_POPUP_BACKDROP_CLASS =
  "fixed inset-0 z-40 hidden items-center justify-center bg-slate-900/20 backdrop-blur-sm";
const EDGE_POPUP_CLASS =
  "w-[260px] min-w-[220px] max-w-[320px] rounded-2xl bg-slate-900/95 px-4 py-3 text-white shadow-2xl";
const EDGE_POPUP_TITLE_CLASS = "text-[13px] font-semibold";
const EDGE_POPUP_META_CLASS = "mt-2 text-xs text-slate-200/80";
const EDGE_POPUP_ACTIONS_CLASS = "mt-3 flex flex-col gap-2";
const EDGE_POPUP_ACTION_CLASS =
  "rounded-lg border border-slate-600/40 bg-slate-800/80 px-2.5 py-2 text-left text-[11px] font-semibold text-white transition hover:bg-slate-700/80";

const setPopupVisible = (backdrop, visible) => {
  if (!backdrop) return;
  backdrop.classList.toggle("hidden", !visible);
  backdrop.classList.toggle("flex", visible);
  backdrop.setAttribute("aria-hidden", visible ? "false" : "true");
};

const bubbleMeasureCanvas = document.createElement("canvas");
const bubbleMeasureCtx = bubbleMeasureCanvas.getContext("2d");

const drawRoundedRect = (ctx, x, y, width, height, radius) => {
  const r = Math.min(radius, width / 2, height / 2);
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + width, y, x + width, y + height, r);
  ctx.arcTo(x + width, y + height, x, y + height, r);
  ctx.arcTo(x, y + height, x, y, r);
  ctx.arcTo(x, y, x + width, y, r);
  ctx.closePath();
};

const measureTextHeight = (metrics, fallback) => {
  const ascent = metrics.actualBoundingBoxAscent;
  const descent = metrics.actualBoundingBoxDescent;
  if (Number.isFinite(ascent) && Number.isFinite(descent)) {
    return ascent + descent;
  }
  return fallback;
};

const drawTextWithSpacing = (ctx, text, centerX, centerY, spacing) => {
  if (!text) return;
  const letters = text.split("");
  const widths = letters.map((letter) => ctx.measureText(letter).width);
  const total =
    widths.reduce((sum, width) => sum + width, 0) +
    spacing * Math.max(0, letters.length - 1);
  let cursor = centerX - total / 2;
  letters.forEach((letter, index) => {
    const width = widths[index] || 0;
    ctx.fillText(letter, cursor + width / 2, centerY);
    cursor += width + spacing;
  });
};

const buildBubbleCanvas = (text, style, pixelRatio) => {
  const fontSize = style.fontSize || 11;
  const fontWeight = style.fontWeight || 600;
  const font = `${fontWeight} ${fontSize}px ${BUBBLE_FONT_STACK}`;
  const paddingX = style.paddingX || 0;
  const paddingY = style.paddingY || 0;
  const borderWidth = style.borderWidth || 0;
  const shadowBlur = style.shadowBlur || 0;
  const shadowPad = shadowBlur ? Math.ceil(shadowBlur * 0.9) : 0;
  const displayText = style.uppercase ? text.toUpperCase() : text;

  bubbleMeasureCtx.font = font;
  const metrics = bubbleMeasureCtx.measureText(displayText);
  const textWidth = metrics.width || 0;
  const textHeight = measureTextHeight(metrics, fontSize);

  let width = style.size || 0;
  let height = style.size || 0;
  if (style.shape !== "circle") {
    width = Math.ceil(textWidth + paddingX * 2 + borderWidth * 2);
    height = Math.ceil(textHeight + paddingY * 2 + borderWidth * 2);
  }

  const canvas = document.createElement("canvas");
  canvas.width = Math.ceil((width + shadowPad * 2) * pixelRatio);
  canvas.height = Math.ceil((height + shadowPad * 2) * pixelRatio);
  const ctx = canvas.getContext("2d");
  ctx.scale(pixelRatio, pixelRatio);

  const centerX = shadowPad + width / 2;
  const centerY = shadowPad + height / 2;
  const shapeX = shadowPad + borderWidth / 2;
  const shapeY = shadowPad + borderWidth / 2;
  const shapeWidth = width - borderWidth;
  const shapeHeight = height - borderWidth;

  ctx.save();
  ctx.fillStyle = style.background;
  ctx.shadowColor = style.shadowColor || "transparent";
  ctx.shadowBlur = shadowBlur;
  if (style.shape === "circle") {
    const radius = shapeWidth / 2;
    ctx.beginPath();
    ctx.arc(centerX, centerY, radius, 0, Math.PI * 2);
    ctx.fill();
  } else {
    drawRoundedRect(ctx, shapeX, shapeY, shapeWidth, shapeHeight, shapeHeight / 2);
    ctx.fill();
  }
  ctx.restore();

  if (borderWidth > 0 && style.borderColor) {
    ctx.save();
    ctx.strokeStyle = style.borderColor;
    ctx.lineWidth = borderWidth;
    if (style.shape === "circle") {
      const radius = shapeWidth / 2;
      ctx.beginPath();
      ctx.arc(centerX, centerY, radius, 0, Math.PI * 2);
      ctx.stroke();
    } else {
      drawRoundedRect(ctx, shapeX, shapeY, shapeWidth, shapeHeight, shapeHeight / 2);
      ctx.stroke();
    }
    ctx.restore();
  }

  ctx.fillStyle = style.textColor;
  ctx.font = font;
  ctx.textBaseline = "middle";
  ctx.textAlign = "center";
  if (style.letterSpacing) {
    drawTextWithSpacing(
      ctx,
      displayText,
      centerX,
      centerY,
      style.letterSpacing * fontSize,
    );
  } else {
    ctx.fillText(displayText, centerX, centerY);
  }

  return {
    canvas,
    width: width + shadowPad * 2,
    height: height + shadowPad * 2,
  };
};

const pinBaseStyleFor = (kind, variant) => {
  if (kind === "flight") {
    return variant === "ground" ? PIN_STYLE_FLIGHT_GROUND : PIN_STYLE_FLIGHT;
  }
  if (kind === "satellite") {
    if (variant === "meo") return PIN_STYLE_SATELLITE_MEO;
    if (variant === "geo") return PIN_STYLE_SATELLITE_GEO;
    if (variant === "unknown") return PIN_STYLE_SATELLITE_UNKNOWN;
    return PIN_STYLE_SATELLITE;
  }
  if (kind === "ship") {
    if (variant === "tanker") return PIN_STYLE_SHIP_TANKER;
    if (variant === "passenger") return PIN_STYLE_SHIP_PASSENGER;
    if (variant === "fishing") return PIN_STYLE_SHIP_FISHING;
    if (variant === "unknown") return PIN_STYLE_SHIP_UNKNOWN;
    return PIN_STYLE_SHIP;
  }
  return PIN_STYLE_DEFAULT;
};

const pinBaseStyleKeyFor = (kind, variant) => {
  return `pin:${kind || "default"}:${variant || "default"}`;
};

const edgeBaseStyleFor = (kind, occluded) => {
  if (occluded) return EDGE_STYLE_OCCLUDED;
  if (kind === "flight") return EDGE_STYLE_FLIGHT;
  if (kind === "satellite") return EDGE_STYLE_SATELLITE;
  if (kind === "ship") return EDGE_STYLE_SHIP;
  return EDGE_STYLE_DEFAULT;
};

const edgeBaseStyleKeyFor = (kind, occluded) => {
  if (occluded) return "edge:occluded";
  return `edge:${kind || "default"}`;
};

class BubbleTextureCache {
  constructor(renderer) {
    this.renderer = renderer;
    this.cache = new Map();
    this.pixelRatio = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
  }

  get(text, styleKey, style) {
    const key = `${styleKey}|${text}`;
    let cached = this.cache.get(key);
    if (cached) return cached;
    const { canvas, width, height } = buildBubbleCanvas(
      text,
      style,
      this.pixelRatio,
    );
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    texture.generateMipmaps = false;
    cached = { texture, width, height };
    this.cache.set(key, cached);
    return cached;
  }
}

class BubblePopup {
  constructor(onAction, onClose) {
    this.onAction = onAction;
    this.onClose = onClose;
    this.popupBackdrop = null;
    this.popup = null;
    this.activeEntityId = null;
    this.initPopup();
  }

  initPopup() {
    const backdrop = document.createElement("div");
    backdrop.className = EDGE_POPUP_BACKDROP_CLASS;
    backdrop.setAttribute("aria-hidden", "true");
    const popup = document.createElement("div");
    popup.className = EDGE_POPUP_CLASS;
    backdrop.appendChild(popup);
    document.body.appendChild(backdrop);
    backdrop.addEventListener("click", () => this.close());
    popup.addEventListener("click", (event) => {
      event.stopPropagation();
      const button = event.target.closest("button[data-action]");
      if (!button || this.activeEntityId === null) return;
      const action = button.dataset.action;
      if (action) this.onAction?.(action, this.activeEntityId);
      this.close();
    });
    this.popupBackdrop = backdrop;
    this.popup = popup;
  }

  openFor(entityId, label, details) {
    if (!this.popup || !this.popupBackdrop) return;
    this.activeEntityId = entityId ?? null;
    const safeLabel = label || "Entity";
    const detailsHtml = details
      ? `<div class="${EDGE_POPUP_META_CLASS}">${details}</div>`
      : "";
    this.popup.innerHTML = `
      <div class="${EDGE_POPUP_TITLE_CLASS}">${safeLabel}</div>
      ${detailsHtml}
      <div class="${EDGE_POPUP_ACTIONS_CLASS}">
        <button class="${EDGE_POPUP_ACTION_CLASS}" data-action="focus">Focus</button>
        <button class="${EDGE_POPUP_ACTION_CLASS}" data-action="details">Details</button>
        <button class="${EDGE_POPUP_ACTION_CLASS}" data-action="task">Task</button>
      </div>
    `;
    setPopupVisible(this.popupBackdrop, true);
  }

  close(silent = false) {
    this.activeEntityId = null;
    setPopupVisible(this.popupBackdrop, false);
    if (!silent) this.onClose?.();
  }
}

class BubbleOverlay {
  constructor(renderer, boundsEl, popup) {
    this.renderer = renderer;
    this.boundsEl = boundsEl;
    this.popup = popup;
    this.onSelect = null;
    this.lodEnabled = true;
    this.scene = new THREE.Scene();
    this.camera = new THREE.OrthographicCamera(0, 1, 1, 0, -1000, 1000);
    this.camera.position.z = 10;
    this.camera.userData.overlay = true;
    this.groups = {
      pins: new THREE.Group(),
      flights: new THREE.Group(),
      satellites: new THREE.Group(),
      ships: new THREE.Group(),
      edges: new THREE.Group(),
    };
    this.scene.add(this.groups.pins);
    this.scene.add(this.groups.flights);
    this.scene.add(this.groups.satellites);
    this.scene.add(this.groups.ships);
    this.scene.add(this.groups.edges);
    this.textureCache = new BubbleTextureCache(renderer);
    this.entries = {
      pins: new Map(),
      flights: new Map(),
      satellites: new Map(),
      ships: new Map(),
      edges: new Map(),
    };
    this.visible = {
      pins: true,
      flights: false,
      satellites: false,
      ships: false,
    };
    this.size = { width: 1, height: 1 };
    this.raycaster = new THREE.Raycaster();
    this.pointer = new THREE.Vector2();
    this.pointerDown = null;
    this.selected = null;
    this.controlsWasEnabled = null;
    if (this.renderer?.setOverlayScene) {
      this.renderer.setOverlayScene(this.scene, this.camera);
    }
    this.bindEvents();
  }

  setLodEnabled(enabled) {
    this.lodEnabled = Boolean(enabled);
    if (!this.lodEnabled) {
      Object.values(this.groups).forEach((group) => {
        if (group) group.visible = false;
      });
      this.clearSelection();
      this.popup?.close(true);
      return;
    }
    this.applyGroupVisibility();
  }

  applyGroupVisibility() {
    if (!this.lodEnabled) return;
    this.groups.pins.visible = this.visible.pins;
    this.groups.flights.visible = this.visible.flights;
    this.groups.satellites.visible = this.visible.satellites;
    this.groups.ships.visible = this.visible.ships;
    this.groups.edges.visible = true;
  }

  bindEvents() {
    const canvas = this.renderer?.canvas;
    if (!canvas) return;
    const onPointerDown = (event) => {
      const hit = this.pick(event);
      if (!hit) return;
      this.pointerDown = { hit, id: event.pointerId };
      const controls = this.renderer?.controls;
      if (controls && this.controlsWasEnabled === null) {
        this.controlsWasEnabled = controls.enabled;
        controls.enabled = false;
      }
      if (canvas.setPointerCapture) {
        canvas.setPointerCapture(event.pointerId);
      }
      event.preventDefault();
      event.stopImmediatePropagation();
    };
    const onPointerUp = (event) => {
      if (!this.pointerDown || this.pointerDown.id !== event.pointerId) return;
      const hit = this.pick(event);
      if (hit && hit.entry === this.pointerDown.hit.entry) {
        this.handleSelection(hit.entry);
      }
      this.pointerDown = null;
      if (canvas.releasePointerCapture) {
        canvas.releasePointerCapture(event.pointerId);
      }
      if (this.controlsWasEnabled !== null && this.renderer?.controls) {
        this.renderer.controls.enabled = this.controlsWasEnabled;
        this.controlsWasEnabled = null;
      }
      event.preventDefault();
      event.stopImmediatePropagation();
    };
    const onPointerCancel = (event) => {
      if (this.pointerDown && canvas.releasePointerCapture) {
        canvas.releasePointerCapture(event.pointerId);
      }
      this.pointerDown = null;
      if (this.controlsWasEnabled !== null && this.renderer?.controls) {
        this.renderer.controls.enabled = this.controlsWasEnabled;
        this.controlsWasEnabled = null;
      }
    };
    canvas.addEventListener("pointerdown", onPointerDown, { capture: true });
    canvas.addEventListener("pointerup", onPointerUp, { capture: true });
    canvas.addEventListener("pointercancel", onPointerCancel, { capture: true });
  }

  resize(width, height) {
    this.size = { width, height };
    this.camera.left = 0;
    this.camera.right = width;
    this.camera.top = height;
    this.camera.bottom = 0;
    this.camera.updateProjectionMatrix();
  }

  setPinsVisible(visible) {
    this.visible.pins = visible;
    if (this.lodEnabled) {
      this.groups.pins.visible = visible;
    }
    if (!visible) this.clearSelectionForGroup("pins");
  }

  setFlightsVisible(visible) {
    this.visible.flights = visible;
    if (this.lodEnabled) {
      this.groups.flights.visible = visible;
    }
    if (!visible) this.clearSelectionForGroup("flights");
  }

  setSatellitesVisible(visible) {
    this.visible.satellites = visible;
    if (this.lodEnabled) {
      this.groups.satellites.visible = visible;
    }
    if (!visible) this.clearSelectionForGroup("satellites");
  }

  setShipsVisible(visible) {
    this.visible.ships = visible;
    if (this.lodEnabled) {
      this.groups.ships.visible = visible;
    }
    if (!visible) this.clearSelectionForGroup("ships");
  }

  clearSelectionForGroup(group) {
    if (!this.selected || this.selected.group !== group) return;
    this.clearSelection();
    this.popup?.close(true);
  }

  clearSelection() {
    if (!this.selected?.entry) return;
    this.applySelection(this.selected.entry, false);
    this.selected = null;
    this.onSelect?.(null);
  }

  pick(event) {
    const canvas = this.renderer?.canvas;
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) return null;
    const x = (event.clientX - rect.left) / rect.width;
    const y = (event.clientY - rect.top) / rect.height;
    this.pointer.set(x * 2 - 1, -(y * 2 - 1));
    this.raycaster.setFromCamera(this.pointer, this.camera);
    const targets = [
      this.groups.edges,
      this.groups.pins,
      this.groups.flights,
      this.groups.satellites,
      this.groups.ships,
    ];
    const hits = this.raycaster.intersectObjects(targets, true);
    const match = hits.find((hit) => hit.object?.userData?.bubble);
    if (!match) return null;
    return { entry: match.object.userData.bubble };
  }

  handleSelection(entry) {
    if (!entry) return;
    if (this.selected && this.selected.entry === entry) {
      this.clearSelection();
      this.popup?.close(true);
      return;
    }
    if (this.selected?.entry) {
      this.applySelection(this.selected.entry, false);
    }
    this.selected = { entry, group: entry.group, entityId: entry.entityId };
    this.applySelection(entry, true);
    this.popup?.openFor(entry.entityId, entry.label, entry.details);
    this.onSelect?.(entry.entityId);
  }

  applySelection(entry, selected) {
    if (!entry) return;
    entry.selected = selected;
    const styleKey = selected
      ? entry.selectedStyleKey
      : entry.baseStyleKey;
    const style = selected ? entry.selectedStyle : entry.baseStyle;
    this.applyStyle(entry, styleKey, style);
  }

  applyStyle(entry, styleKey, style) {
    const text = entry.text || "";
    const textureKey = `${styleKey}|${text}`;
    if (entry.textureKey === textureKey) return;
    const { texture, width, height } = this.textureCache.get(
      text,
      styleKey,
      style,
    );
    entry.sprite.material.map = texture;
    entry.sprite.material.needsUpdate = true;
    entry.sprite.scale.set(width, height, 1);
    entry.textureKey = textureKey;
  }

  ensureEntry(map, groupName, entityId, data) {
    let entry = map.get(entityId);
    if (!entry) {
      const material = new THREE.SpriteMaterial({
        transparent: true,
        depthTest: false,
        depthWrite: false,
      });
      const sprite = new THREE.Sprite(material);
      sprite.renderOrder = groupName === "edges" ? 230 : 210;
      entry = {
        entityId,
        group: groupName,
        sprite,
        text: "",
        label: "",
        details: "",
        baseStyleKey: "",
        baseStyle: null,
        selectedStyleKey: groupName === "edges" ? "edge:selected" : "pin:selected",
        selectedStyle: groupName === "edges" ? EDGE_STYLE_SELECTED : PIN_STYLE_SELECTED,
        textureKey: "",
        selected: false,
      };
      sprite.userData.bubble = entry;
      this.groups[groupName].add(sprite);
      map.set(entityId, entry);
    }
    entry.text = data.text || "";
    entry.label = data.label || "Entity";
    entry.details = data.details || "";
    entry.baseStyleKey = data.baseStyleKey;
    entry.baseStyle = data.baseStyle;
    entry.selectedStyleKey =
      groupName === "edges" ? "edge:selected" : "pin:selected";
    entry.selectedStyle = groupName === "edges" ? EDGE_STYLE_SELECTED : PIN_STYLE_SELECTED;
    this.applySelection(entry, data.selected);
    return entry;
  }

  positionEntry(entry, screenX, screenY) {
    entry.sprite.position.set(screenX, this.size.height - screenY, 0);
  }

  hideEntry(entry) {
    if (!entry) return;
    entry.sprite.visible = false;
    if (this.selected?.entry === entry) {
      this.clearSelection();
      this.popup?.close(true);
    }
  }

  pruneEntries(map, seen) {
    for (const [entity, entry] of map.entries()) {
      if (seen.has(entity)) continue;
      if (this.selected?.entry === entry) {
        this.clearSelection();
        this.popup?.close(true);
      }
      this.groups[entry.group].remove(entry.sprite);
      entry.sprite.material.dispose();
      map.delete(entity);
    }
  }

  syncPins(entities, store) {
    if (!this.lodEnabled) return;
    const seen = new Set();
    if (!this.visible.pins) {
      this.groups.pins.visible = false;
    } else {
      this.groups.pins.visible = true;
    }
    const bounds = this.boundsEl?.getBoundingClientRect?.();
    const clamp = bounds || {
      left: 0,
      top: 0,
      right: this.size.width,
      bottom: this.size.height,
    };
    const pad = 18;
    forEachEntity(entities, (entity) => {
      const pin = store.getComponent(entity, "Pin");
      const meta = store.getComponent(entity, "Meta");
      if (!pin) return;
      if (meta?.kind === "flight" || meta?.kind === "satellite" || meta?.kind === "ship") {
        return;
      }
      const geo = store.getComponent(entity, "Geo");
      if (!geo || !this.renderer) return;
      const pos = positionForEntity(
        entity,
        this.renderer,
        geo,
        this.renderer.markerAltitude + 1.5,
      );
      if (!pos) return;
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;
      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      const visible = this.visible.pins && screen.visible && withinBounds;
      const baseStyle = pinBaseStyleFor("default", "default");
      const baseStyleKey = pinBaseStyleKeyFor("default", "default");
      const entry = this.ensureEntry(this.entries.pins, "pins", entity, {
        text: pin.label || "Entity",
        label: pin.label || meta?.data?.name || "Entity",
        details: "",
        baseStyleKey,
        baseStyle,
        selected:
          this.selected?.entityId === entity && this.selected.group === "pins",
      });
      entry.sprite.visible = visible;
      if (visible) {
        this.positionEntry(entry, screen.x, screen.y);
      } else {
        this.hideEntry(entry);
      }
      seen.add(entity);
    });
    this.pruneEntries(this.entries.pins, seen);
  }

  syncFlights(entities, store) {
    if (!this.lodEnabled) return;
    const seen = new Set();
    if (!this.visible.flights) {
      this.groups.flights.visible = false;
    } else {
      this.groups.flights.visible = true;
    }
    const bounds = this.boundsEl?.getBoundingClientRect?.();
    const clamp = bounds || {
      left: 0,
      top: 0,
      right: this.size.width,
      bottom: this.size.height,
    };
    const pad = 22;
    forEachEntity(entities, (entity) => {
      const flight = store.getComponent(entity, "Flight");
      if (!flight) return;
      const geo = store.getComponent(entity, "Geo");
      if (!geo || !this.renderer) return;
      const altitude = altitudeForFlight(flight);
      const pos = positionForEntity(
        entity,
        this.renderer,
        geo,
        this.renderer.markerAltitude + altitude,
      );
      if (!pos) return;
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;
      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      const visible = this.visible.flights && screen.visible && withinBounds;
      const label = formatFlightLabel(flight);
      const details = formatFlightDetails(flight);
      const variant = flight.on_ground ? "ground" : "air";
      const baseStyle = pinBaseStyleFor("flight", variant);
      const baseStyleKey = pinBaseStyleKeyFor("flight", variant);
      const entry = this.ensureEntry(this.entries.flights, "flights", entity, {
        text: label,
        label,
        details,
        baseStyleKey,
        baseStyle,
        selected:
          this.selected?.entityId === entity && this.selected.group === "flights",
      });
      entry.sprite.visible = visible;
      if (visible) {
        this.positionEntry(entry, screen.x, screen.y);
      } else {
        this.hideEntry(entry);
      }
      seen.add(entity);
    });
    this.pruneEntries(this.entries.flights, seen);
  }

  syncSatellites(entities, store) {
    if (!this.lodEnabled) return;
    const seen = new Set();
    if (!this.visible.satellites) {
      this.groups.satellites.visible = false;
    } else {
      this.groups.satellites.visible = true;
    }
    const bounds = this.boundsEl?.getBoundingClientRect?.();
    const clamp = bounds || {
      left: 0,
      top: 0,
      right: this.size.width,
      bottom: this.size.height,
    };
    const pad = 22;
    forEachEntity(entities, (entity) => {
      const satellite = store.getComponent(entity, "Satellite");
      if (!satellite) return;
      const geo = store.getComponent(entity, "Geo");
      if (!geo || !this.renderer) return;
      const altitude = altitudeForSatellite(satellite);
      const pos = positionForEntity(
        entity,
        this.renderer,
        geo,
        this.renderer.markerAltitude + altitude,
      );
      if (!pos) return;
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;
      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      const visible = this.visible.satellites && screen.visible && withinBounds;
      const label = formatSatelliteLabel(satellite);
      const details = formatSatelliteDetails(satellite);
      const variant = orbitBandForSatellite(satellite);
      const baseStyle = pinBaseStyleFor("satellite", variant);
      const baseStyleKey = pinBaseStyleKeyFor("satellite", variant);
      const entry = this.ensureEntry(
        this.entries.satellites,
        "satellites",
        entity,
        {
          text: label,
          label,
          details,
          baseStyleKey,
          baseStyle,
          selected:
            this.selected?.entityId === entity &&
            this.selected.group === "satellites",
        },
      );
      entry.sprite.visible = visible;
      if (visible) {
        this.positionEntry(entry, screen.x, screen.y);
      } else {
        this.hideEntry(entry);
      }
      seen.add(entity);
    });
    this.pruneEntries(this.entries.satellites, seen);
  }

  syncShips(entities, store) {
    if (!this.lodEnabled) return;
    const seen = new Set();
    if (!this.visible.ships) {
      this.groups.ships.visible = false;
    } else {
      this.groups.ships.visible = true;
    }
    const bounds = this.boundsEl?.getBoundingClientRect?.();
    const clamp = bounds || {
      left: 0,
      top: 0,
      right: this.size.width,
      bottom: this.size.height,
    };
    const pad = 22;
    const baseAltitude = shipBaseAltitude(this.renderer);
    forEachEntity(entities, (entity) => {
      const ship = store.getComponent(entity, "Ship");
      if (!ship) return;
      const geo = store.getComponent(entity, "Geo");
      if (!geo || !this.renderer) return;
      const pos = positionForEntity(
        entity,
        this.renderer,
        geo,
        baseAltitude + altitudeForShip(ship),
      );
      if (!pos) return;
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;
      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      const visible = this.visible.ships && screen.visible && withinBounds;
      const label = formatShipLabel(ship);
      const details = formatShipDetails(ship);
      const variant = vesselGroupForShip(ship);
      const baseStyle = pinBaseStyleFor("ship", variant);
      const baseStyleKey = pinBaseStyleKeyFor("ship", variant);
      const entry = this.ensureEntry(this.entries.ships, "ships", entity, {
        text: label,
        label,
        details,
        baseStyleKey,
        baseStyle,
        selected:
          this.selected?.entityId === entity && this.selected.group === "ships",
      });
      entry.sprite.visible = visible;
      if (visible) {
        this.positionEntry(entry, screen.x, screen.y);
      } else {
        this.hideEntry(entry);
      }
      seen.add(entity);
    });
    this.pruneEntries(this.entries.ships, seen);
  }

  syncEdges(entities, store) {
    if (!this.lodEnabled) return;
    const seen = new Set();
    const clamp = {
      left: 0,
      top: 0,
      right: this.size.width,
      bottom: this.size.height,
      width: this.size.width,
      height: this.size.height,
    };
    const pad = 22;
    const centerX = clamp.left + clamp.width / 2;
    const centerY = clamp.top + clamp.height / 2;
    const edgeX = clamp.width / 2 - pad;
    const edgeY = clamp.height / 2 - pad;
    forEachEntity(entities, (entity) => {
      const geo = store.getComponent(entity, "Geo");
      const meta = store.getComponent(entity, "Meta");
      const pin = store.getComponent(entity, "Pin");
      if (!geo || !meta || !pin || !this.renderer) return;
      const pos = positionForEntity(
        entity,
        this.renderer,
        geo,
        this.renderer.markerAltitude + 2.5,
      );
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;
      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      if (screen.visible && withinBounds) {
        const existing = this.entries.edges.get(entity);
        if (existing) this.hideEntry(existing);
        return;
      }
      const dx = screen.x - centerX;
      const dy = screen.y - centerY;
      const halfW = edgeX;
      const halfH = edgeY;
      const safeDx = Math.abs(dx) < 1 ? (dx >= 0 ? 1 : -1) : dx;
      const safeDy = Math.abs(dy) < 1 ? (dy >= 0 ? 1 : -1) : dy;
      const scale = Math.min(halfW / Math.abs(safeDx), halfH / Math.abs(safeDy));
      let x = centerX + safeDx * scale;
      let y = centerY + safeDy * scale;
      const hitVertical = Math.abs(safeDx) * halfH >= Math.abs(safeDy) * halfW;
      if (hitVertical) {
        x = centerX + Math.sign(safeDx) * halfW;
      } else {
        y = centerY + Math.sign(safeDy) * halfH;
      }
      const symbol = collapseLabel(pin.label) || edgeSymbolFor(meta);
      const label =
        pin.label || meta.data?.name || meta.data?.summary || meta.kind || "Entity";
      const details =
        meta.kind === "flight"
          ? formatFlightDetails(meta.data)
          : meta.kind === "satellite"
            ? formatSatelliteDetails(meta.data)
            : meta.kind === "ship"
              ? formatShipDetails(meta.data)
              : "";
      const baseStyle = edgeBaseStyleFor(meta.kind, screen.behind);
      const baseStyleKey = edgeBaseStyleKeyFor(meta.kind, screen.behind);
      const entry = this.ensureEntry(this.entries.edges, "edges", entity, {
        text: symbol,
        label,
        details,
        baseStyleKey,
        baseStyle,
        selected:
          this.selected?.entityId === entity && this.selected.group === "edges",
      });
      entry.sprite.visible = true;
      this.positionEntry(entry, x, y);
      seen.add(entity);
    });
    this.pruneEntries(this.entries.edges, seen);
  }
}

export { BubbleOverlay, BubblePopup, FlightOverlay, SatelliteOverlay, ShipOverlay };

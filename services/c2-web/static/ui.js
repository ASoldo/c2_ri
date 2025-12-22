import * as THREE from "/static/vendor/three.module.js";
import { OrbitControls } from "/static/vendor/OrbitControls.js";

const els = {
  apiStatus: document.getElementById("api-status"),
  apiDot: document.getElementById("api-dot"),
  streamStatus: document.getElementById("stream-status"),
  wsStatus: document.getElementById("ws-status"),
  runtimeStats: document.getElementById("runtime-stats"),
  cameraStats: document.getElementById("camera-stats"),
  tileStatus: document.getElementById("tile-status"),
  board: document.getElementById("board"),
  layerStack: document.getElementById("layer-stack"),
  map2d: document.getElementById("map-2d"),
  map3d: document.getElementById("map-3d"),
  pinLayer: document.getElementById("pin-layer"),
  flightLayer: document.getElementById("flight-layer"),
  satelliteLayer: document.getElementById("satellite-layer"),
  shipLayer: document.getElementById("ship-layer"),
  edgeLayer: document.getElementById("edge-layer"),
  dockLeft: document.getElementById("dock-left"),
  dockRight: document.getElementById("dock-right"),
  flightProviderLabel: document.getElementById("flight-provider-label"),
  satelliteProviderLabel: document.getElementById("satellite-provider-label"),
  shipProviderLabel: document.getElementById("ship-provider-label"),
};

const partialEls = Array.from(document.querySelectorAll("[data-partial]"));

const DEFAULT_TILE_PROVIDERS = {
  osm: {
    name: "OSM Standard",
    url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
    proxy: true,
    minZoom: 0,
    maxZoom: 19,
  },
  hot: {
    name: "OSM Humanitarian",
    url: "https://a.tile.openstreetmap.fr/hot/{z}/{x}/{y}.png",
    proxy: true,
    minZoom: 0,
    maxZoom: 19,
  },
  opentopo: {
    name: "OpenTopoMap",
    url: "https://tile.opentopomap.org/{z}/{x}/{y}.png",
    proxy: true,
    minZoom: 0,
    maxZoom: 17,
  },
  nasa: {
    name: "NASA Blue Marble",
    url: "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_ShadedRelief/default/2013-12-01/GoogleMapsCompatible_Level8/{z}/{y}/{x}.jpg",
    proxy: true,
    minZoom: 0,
    maxZoom: 8,
  },
};

const buildTileConfig = () => {
  const config = window.C2_TILE_CONFIG || {};
  const providers = { ...DEFAULT_TILE_PROVIDERS, ...(config.providers || {}) };
  const normalized = {};
  Object.entries(providers).forEach(([id, provider]) => {
    if (!provider || !provider.url) return;
    const remoteUrl = provider.url;
    const proxy =
      provider.proxy !== false && /^https?:\/\//i.test(remoteUrl || "");
    const resolvedUrl = proxy
      ? `/ui/tiles/${id}/{z}/{x}/{y}`
      : remoteUrl;
    normalized[id] = {
      id,
      name: provider.name || id,
      url: resolvedUrl,
      remoteUrl,
      proxy,
      minZoom: Number.isFinite(provider.minZoom) ? provider.minZoom : 0,
      maxZoom: Number.isFinite(provider.maxZoom) ? provider.maxZoom : 19,
      zoomBias: Number.isFinite(provider.zoomBias) ? provider.zoomBias : 0,
    };
  });
  const order = Array.isArray(config.order)
    ? config.order.filter((id) => normalized[id])
    : Object.keys(normalized);
  const saved = window.localStorage?.getItem?.("c2.tileProvider");
  const activeProvider = config.activeProvider || saved || order[0] || null;
  const maxTiles = Number.isFinite(config.maxTiles) ? config.maxTiles : 220;
  const maxCache = Number.isFinite(config.maxCache)
    ? config.maxCache
    : Math.max(256, maxTiles * 3);
  return {
    providers: normalized,
    order,
    activeProvider,
    maxTiles,
    maxCache,
  };
};

const TILE_CONFIG = buildTileConfig();

const DEFAULT_WEATHER_FIELDS = [
  "IMERG_Precipitation_Rate",
  "AIRS_Precipitation_Day",
  "MODIS_Terra_Cloud_Fraction_Day",
  "MODIS_Terra_Cloud_Top_Temp_Day",
  "MODIS_Terra_Cloud_Top_Pressure_Day",
  "MODIS_Terra_Cloud_Top_Height_Day",
  "MERRA2_2m_Air_Temperature_Monthly",
];

const buildWeatherConfig = () => {
  const config = window.C2_WEATHER_CONFIG || {};
  let fields = Array.isArray(config.fields) && config.fields.length
    ? config.fields.filter(Boolean)
    : DEFAULT_WEATHER_FIELDS.slice();
  if (!fields.length) fields = DEFAULT_WEATHER_FIELDS.slice();
  const defaultField = fields.includes(config.defaultField)
    ? config.defaultField
    : fields[0];
  const defaultTime = config.defaultTime || "default";
  const defaultFormat = config.defaultFormat || "png";
  const defaultOpacity = Number.isFinite(config.defaultOpacity)
    ? config.defaultOpacity
    : 0.55;
  const maxTiles = Number.isFinite(config.maxTiles) ? config.maxTiles : 60;
  const updateIntervalMs = Number.isFinite(config.updateIntervalMs)
    ? config.updateIntervalMs
    : 900;
  const maxInFlight = Number.isFinite(config.maxInFlight) ? config.maxInFlight : 3;
  const minZoom = Number.isFinite(config.minZoom) ? config.minZoom : 0;
  const maxZoom = Number.isFinite(config.maxZoom) ? config.maxZoom : 6;
  return {
    enabled: Boolean(config.enabled),
    fields,
    defaultField,
    defaultTime,
    defaultFormat,
    defaultOpacity,
    maxTiles,
    updateIntervalMs,
    maxInFlight,
    minZoom,
    maxZoom,
  };
};

const WEATHER_CONFIG = buildWeatherConfig();

const DEFAULT_FLIGHT_CONFIG = {
  enabled: true,
  provider: "opensky",
  updateIntervalMs: 5000,
  minIntervalMs: 3500,
  maxFlights: 80,
  trailPoints: 24,
  trailMaxAgeMs: 240000,
  spanMinDeg: 8,
  spanMaxDeg: 60,
  altitudeScale: 0.08,
  source: "OpenSky",
  sample: true,
};

const buildFlightConfig = () => {
  const config = window.C2_FLIGHT_CONFIG || {};
  return {
    enabled: config.enabled !== undefined ? Boolean(config.enabled) : DEFAULT_FLIGHT_CONFIG.enabled,
    provider: config.provider || DEFAULT_FLIGHT_CONFIG.provider,
    updateIntervalMs: Number.isFinite(config.updateIntervalMs)
      ? config.updateIntervalMs
      : DEFAULT_FLIGHT_CONFIG.updateIntervalMs,
    minIntervalMs: Number.isFinite(config.minIntervalMs)
      ? config.minIntervalMs
      : DEFAULT_FLIGHT_CONFIG.minIntervalMs,
    maxFlights: Number.isFinite(config.maxFlights)
      ? config.maxFlights
      : DEFAULT_FLIGHT_CONFIG.maxFlights,
    trailPoints: Number.isFinite(config.trailPoints)
      ? config.trailPoints
      : DEFAULT_FLIGHT_CONFIG.trailPoints,
    trailMaxAgeMs: Number.isFinite(config.trailMaxAgeMs)
      ? config.trailMaxAgeMs
      : DEFAULT_FLIGHT_CONFIG.trailMaxAgeMs,
    spanMinDeg: Number.isFinite(config.spanMinDeg)
      ? config.spanMinDeg
      : DEFAULT_FLIGHT_CONFIG.spanMinDeg,
    spanMaxDeg: Number.isFinite(config.spanMaxDeg)
      ? config.spanMaxDeg
      : DEFAULT_FLIGHT_CONFIG.spanMaxDeg,
    altitudeScale: Number.isFinite(config.altitudeScale)
      ? config.altitudeScale
      : DEFAULT_FLIGHT_CONFIG.altitudeScale,
    source: config.source || DEFAULT_FLIGHT_CONFIG.source,
    sample: config.sample !== undefined ? Boolean(config.sample) : DEFAULT_FLIGHT_CONFIG.sample,
  };
};

const FLIGHT_CONFIG = buildFlightConfig();

const DEFAULT_SATELLITE_CONFIG = {
  enabled: true,
  provider: "celestrak",
  updateIntervalMs: 8000,
  maxSatellites: 120,
  altitudeScale: 0.018,
  altitudeMin: 4,
  altitudeMax: 90,
  source: "CelesTrak",
  sample: true,
};

const buildSatelliteConfig = () => {
  const config = window.C2_SATELLITE_CONFIG || {};
  return {
    enabled:
      config.enabled !== undefined ? Boolean(config.enabled) : DEFAULT_SATELLITE_CONFIG.enabled,
    provider: config.provider || DEFAULT_SATELLITE_CONFIG.provider,
    updateIntervalMs: Number.isFinite(config.updateIntervalMs)
      ? config.updateIntervalMs
      : DEFAULT_SATELLITE_CONFIG.updateIntervalMs,
    maxSatellites: Number.isFinite(config.maxSatellites)
      ? config.maxSatellites
      : DEFAULT_SATELLITE_CONFIG.maxSatellites,
    altitudeScale: Number.isFinite(config.altitudeScale)
      ? config.altitudeScale
      : DEFAULT_SATELLITE_CONFIG.altitudeScale,
    altitudeMin: Number.isFinite(config.altitudeMin)
      ? config.altitudeMin
      : DEFAULT_SATELLITE_CONFIG.altitudeMin,
    altitudeMax: Number.isFinite(config.altitudeMax)
      ? config.altitudeMax
      : DEFAULT_SATELLITE_CONFIG.altitudeMax,
    source: config.source || DEFAULT_SATELLITE_CONFIG.source,
    sample: config.sample !== undefined ? Boolean(config.sample) : DEFAULT_SATELLITE_CONFIG.sample,
  };
};

const SATELLITE_CONFIG = buildSatelliteConfig();

const DEFAULT_SHIP_CONFIG = {
  enabled: true,
  provider: "arcgis",
  updateIntervalMs: 9000,
  maxShips: 200,
  spanMinDeg: 6,
  spanMaxDeg: 70,
  altitude: 0.12,
  source: "ArcGIS ShipPositions",
  sample: true,
};

const buildShipConfig = () => {
  const config = window.C2_SHIP_CONFIG || {};
  return {
    enabled: config.enabled !== undefined ? Boolean(config.enabled) : DEFAULT_SHIP_CONFIG.enabled,
    provider: config.provider || DEFAULT_SHIP_CONFIG.provider,
    updateIntervalMs: Number.isFinite(config.updateIntervalMs)
      ? config.updateIntervalMs
      : DEFAULT_SHIP_CONFIG.updateIntervalMs,
    maxShips: Number.isFinite(config.maxShips)
      ? config.maxShips
      : DEFAULT_SHIP_CONFIG.maxShips,
    spanMinDeg: Number.isFinite(config.spanMinDeg)
      ? config.spanMinDeg
      : DEFAULT_SHIP_CONFIG.spanMinDeg,
    spanMaxDeg: Number.isFinite(config.spanMaxDeg)
      ? config.spanMaxDeg
      : DEFAULT_SHIP_CONFIG.spanMaxDeg,
    altitude: Number.isFinite(config.altitude) ? config.altitude : DEFAULT_SHIP_CONFIG.altitude,
    source: config.source || DEFAULT_SHIP_CONFIG.source,
    sample: config.sample !== undefined ? Boolean(config.sample) : DEFAULT_SHIP_CONFIG.sample,
  };
};

const SHIP_CONFIG = buildShipConfig();

const setDot = (state) => {
  if (!els.apiDot) return;
  els.apiDot.classList.remove("ok", "warn", "error");
  if (state) els.apiDot.classList.add(state);
};

const swapHtml = (targetId, html) => {
  const el = document.getElementById(targetId);
  if (!el) return;
  el.innerHTML = html;
};

const refreshPartials = async () => {
  await Promise.all(
    partialEls.map(async (el) => {
      const url = el.dataset.partial;
      if (!url) return;
      try {
        const response = await fetch(url, { cache: "no-store" });
        if (!response.ok) return;
        const html = await response.text();
        el.innerHTML = html;
      } catch {
        // ignore partial refresh errors
      }
    }),
  );
};

const updateStatus = async () => {
  if (!els.apiStatus) return;
  try {
    const response = await fetch("/ui/status", { cache: "no-store" });
    if (!response.ok) throw new Error("status fetch failed");
    const data = await response.json();
    els.apiStatus.textContent = `API: ${data.service} (${data.environment})`;
    setDot("ok");
  } catch {
    els.apiStatus.textContent = "API: unavailable";
    setDot("warn");
  }
};

const applyPartialBatch = (payload) => {
  if (!payload || !Array.isArray(payload.fragments)) return;
  payload.fragments.forEach((fragment) => {
    if (!fragment || !fragment.target) return;
    swapHtml(fragment.target, fragment.html || "");
  });
};

const startSse = (bus) => {
  if (!els.streamStatus || !window.EventSource) return;
  const source = new EventSource("/ui/stream/sse");
  els.streamStatus.textContent = "SSE: connecting";
  source.addEventListener("partials", (event) => {
    els.streamStatus.textContent = "SSE: live";
    const payload = JSON.parse(event.data || "{}");
    applyPartialBatch(payload);
  });
  source.addEventListener("entities", (event) => {
    const payload = JSON.parse(event.data || "{}");
    bus.emit("entities:update", payload);
  });
  source.addEventListener("error", () => {
    els.streamStatus.textContent = "SSE: reconnecting";
  });
  source.onmessage = (event) => {
    bus.emit("sse:message", event.data);
  };
  source.onerror = () => {
    els.streamStatus.textContent = "SSE: reconnecting";
  };
};

const startWs = (bus) => {
  if (!els.wsStatus || !window.WebSocket) return;
  const scheme = window.location.protocol === "https:" ? "wss" : "ws";
  const socket = new WebSocket(`${scheme}://${window.location.host}/ui/stream/ws`);
  els.wsStatus.textContent = "WS: connecting";
  socket.onopen = () => {
    els.wsStatus.textContent = "WS: live";
  };
  socket.onmessage = (event) => {
    try {
      const message = JSON.parse(event.data || "{}");
      if (message.kind === "partials") {
        applyPartialBatch(message.payload);
      } else if (message.kind === "entities") {
        bus.emit("entities:update", message.payload);
      } else {
        bus.emit("ws:message", message);
      }
    } catch {
      // ignore parse errors
    }
  };
  socket.onclose = () => {
    els.wsStatus.textContent = "WS: reconnecting";
    setTimeout(() => startWs(bus), 3000);
  };
  socket.onerror = () => {
    els.wsStatus.textContent = "WS: reconnecting";
  };
};

const fetchEntities = async (bus) => {
  try {
    const response = await fetch("/ui/entities", { cache: "no-store" });
    if (!response.ok) return;
    const payload = await response.json();
    bus.emit("entities:update", payload);
  } catch {
    // ignore entity fetch errors
  }
};

class EventBus {
  constructor() {
    this.handlers = new Map();
  }

  on(event, handler) {
    if (!this.handlers.has(event)) this.handlers.set(event, []);
    this.handlers.get(event).push(handler);
  }

  emit(event, payload) {
    const handlers = this.handlers.get(event) || [];
    handlers.forEach((handler) => handler(payload));
  }
}

class World {
  constructor() {
    this.nextId = 1;
    this.entities = new Set();
    this.components = new Map();
  }

  createEntity() {
    const id = this.nextId++;
    this.entities.add(id);
    return id;
  }

  removeEntity(entity) {
    this.entities.delete(entity);
    for (const map of this.components.values()) {
      map.delete(entity);
    }
  }

  addComponent(entity, type, data) {
    if (!this.components.has(type)) this.components.set(type, new Map());
    this.components.get(type).set(entity, data);
  }

  removeComponent(entity, type) {
    this.components.get(type)?.delete(entity);
  }

  getComponent(entity, type) {
    return this.components.get(type)?.get(entity);
  }

  query(types) {
    const results = [];
    for (const entity of this.entities) {
      if (types.every((type) => this.components.get(type)?.has(entity))) {
        results.push(entity);
      }
    }
    return results;
  }
}

class BoardView {
  constructor(boardEl, canvas2d) {
    this.boardEl = boardEl;
    this.canvas = canvas2d;
    this.ctx = canvas2d?.getContext("2d");
    this.offset = { x: 0, y: 0 };
    this.zoom = 1.0;
    this.isPanning = false;
    this.last = { x: 0, y: 0 };
  }

  resize() {
    if (!this.canvas || !this.boardEl) return;
    const rect = this.boardEl.getBoundingClientRect();
    this.canvas.width = rect.width * devicePixelRatio;
    this.canvas.height = rect.height * devicePixelRatio;
    this.ctx?.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
  }

  bindInputs() {
    if (!this.boardEl) return;
    this.boardEl.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      this.isPanning = true;
      this.last = { x: event.clientX, y: event.clientY };
    });
    window.addEventListener("pointerup", () => {
      this.isPanning = false;
    });
    window.addEventListener("pointermove", (event) => {
      if (!this.isPanning) return;
      const dx = event.clientX - this.last.x;
      const dy = event.clientY - this.last.y;
      this.offset.x += dx;
      this.offset.y += dy;
      this.last = { x: event.clientX, y: event.clientY };
    });
    this.boardEl.addEventListener("wheel", (event) => {
      event.preventDefault();
      const delta = Math.sign(event.deltaY) * -0.1;
      this.zoom = Math.min(4, Math.max(0.4, this.zoom + delta));
    });
  }

  worldToScreen(point) {
    if (!this.boardEl) return { x: 0, y: 0 };
    const rect = this.boardEl.getBoundingClientRect();
    return {
      x: rect.width / 2 + point.x * this.zoom + this.offset.x,
      y: rect.height / 2 + point.y * this.zoom + this.offset.y,
    };
  }

  drawGrid() {
    if (!this.ctx || !this.boardEl || !els.map2d) return;
    if (els.map2d.style.display === "none") return;
    const rect = this.boardEl.getBoundingClientRect();
    this.ctx.clearRect(0, 0, rect.width, rect.height);
    const spacing = 48 * this.zoom;
    this.ctx.strokeStyle = "rgba(15, 23, 42, 0.08)";
    this.ctx.lineWidth = 1;
    for (let x = this.offset.x % spacing; x < rect.width; x += spacing) {
      this.ctx.beginPath();
      this.ctx.moveTo(x, 0);
      this.ctx.lineTo(x, rect.height);
      this.ctx.stroke();
    }
    for (let y = this.offset.y % spacing; y < rect.height; y += spacing) {
      this.ctx.beginPath();
      this.ctx.moveTo(0, y);
      this.ctx.lineTo(rect.width, y);
      this.ctx.stroke();
    }
  }
}

const clampLat = (lat) => Math.max(-85.05112878, Math.min(85.05112878, lat));
const TWO_PI = Math.PI * 2;

const tileXForLon = (lon, zoom) => {
  const n = 2 ** zoom;
  return Math.floor(((lon + 180) / 360) * n);
};

const mercatorYForLat = (lat) => {
  const rad = (clampLat(lat) * Math.PI) / 180;
  return (1 - Math.log(Math.tan(rad) + 1 / Math.cos(rad)) / Math.PI) / 2;
};

const tileYForLat = (lat, zoom) => {
  const n = 2 ** zoom;
  const value = mercatorYForLat(lat);
  const y = Math.floor(value * n);
  return Math.max(0, Math.min(n - 1, y));
};

const tileBounds = (x, y, zoom) => {
  const n = 2 ** zoom;
  const lonWest = (x / n) * 360 - 180;
  const lonEast = ((x + 1) / n) * 360 - 180;
  const latNorth = (180 / Math.PI) * Math.atan(Math.sinh(Math.PI * (1 - (2 * y) / n)));
  const latSouth =
    (180 / Math.PI) * Math.atan(Math.sinh(Math.PI * (1 - (2 * (y + 1)) / n)));
  return { latNorth, latSouth, lonWest, lonEast };
};

const AXIS_Y = new THREE.Vector3(0, 1, 0);

const sphereToGeo = (point) => {
  const radius = Math.max(point.length(), 1);
  const phi = Math.acos(point.y / radius);
  let theta = Math.atan2(point.z, point.x);
  if (theta < 0) theta += TWO_PI;
  const lat = 90 - (phi * 180) / Math.PI;
  const lon = (theta * 180) / Math.PI - 180;
  return { lat, lon };
};

const sphereToGeoTile = (point) => {
  const radius = Math.max(point.length(), 1);
  const theta = Math.acos(point.y / radius);
  let phi = Math.atan2(point.z, -point.x);
  if (phi < 0) phi += TWO_PI;
  const lat = 90 - (theta * 180) / Math.PI;
  const lon = (phi * 180) / Math.PI - 180;
  return { lat, lon };
};

class TileManager {
  constructor(scene, radius, renderer, rotationY = 0) {
    this.scene = scene;
    this.radius = radius;
    this.renderer = renderer;
    this.group = new THREE.Group();
    this.rotationY = rotationY;
    this.group.rotation.y = rotationY;
    this.group.visible = false;
    this.group.renderOrder = 10;
    this.scene.add(this.group);
    this.tiles = new Map();
    this.pending = new Set();
    this.queue = [];
    this.inFlight = 0;
    this.desiredKeys = new Set();
    this.forceUpdate = false;
    this.maxInFlight = 8;
    this.provider = null;
    this.maxTiles = TILE_CONFIG.maxTiles;
    this.maxCache = TILE_CONFIG.maxCache;
    this.baseDistance = 1;
    this.zoom = null;
    this.lastUpdate = 0;
    this.lastDirection = new THREE.Vector3();
    this.lastDistance = 0;
    this.ray = new THREE.Ray();
    this.tmpVec = new THREE.Vector3();
    this.tmpPoint = new THREE.Vector3();
    this.loader = new THREE.TextureLoader();
    this.loader.crossOrigin = "anonymous";
  }

  setBaseDistance(distance) {
    if (Number.isFinite(distance) && distance > 0) {
      this.baseDistance = distance;
    }
  }

  setProvider(provider) {
    this.provider = provider || null;
    this.zoom = null;
    this.group.visible = Boolean(this.provider);
    this.group.renderOrder = this.provider?.renderOrder ?? 10;
    if (this.loader) {
      this.loader.crossOrigin = this.provider?.proxy ? null : "anonymous";
    }
    this.clear();
  }

  markDirty() {
    this.forceUpdate = true;
  }

  clear() {
    for (const tile of this.tiles.values()) {
      tile.mesh?.removeFromParent();
      tile.texture?.dispose();
      tile.geometry?.dispose();
      tile.material?.dispose();
    }
    this.tiles.clear();
    this.pending.clear();
  }

  pickZoom(camera, size) {
    if (!this.provider || !camera?.isPerspectiveCamera) return this.provider?.minZoom ?? 0;
    if (!size?.width || !size?.height) return this.provider?.minZoom ?? 0;
    const distance = camera.position.length();
    const depth = Math.max(1, distance - this.radius);
    const fovV = THREE.MathUtils.degToRad(camera.fov);
    const fovH = 2 * Math.atan(Math.tan(fovV / 2) * camera.aspect);
    const visibleWidth = 2 * depth * Math.tan(fovH / 2);
    const visibleHeight = 2 * depth * Math.tan(fovV / 2);
    const degWidth = (visibleWidth / this.radius) * (180 / Math.PI);
    const degHeight = (visibleHeight / this.radius) * (180 / Math.PI);
    const tileDegWidth = degWidth * (256 / size.width);
    const tileDegHeight = degHeight * (256 / size.height);
    const tileDeg = Math.max(tileDegWidth, tileDegHeight);
    let zoom = Math.round(Math.log2(360 / Math.max(0.0001, tileDeg)));
    zoom += this.provider.zoomBias || 0;
    return Math.min(this.provider.maxZoom, Math.max(this.provider.minZoom, zoom));
  }

  update(camera, size) {
    if (!this.provider || !camera || !size?.width || !size?.height) return;
    const now = performance.now();
    const dir = this.tmpVec.copy(camera.position).normalize();
    const distance = camera.position.length();
    let zoom = this.pickZoom(camera, size);
    const zoomChanged = zoom !== this.zoom;
    const distanceDelta = Math.abs(distance - this.lastDistance);
    const minDistanceDelta = Math.max(0.08, distance * 0.0015);
    const interval = Number.isFinite(this.provider?.updateIntervalMs)
      ? this.provider.updateIntervalMs
      : zoom >= 16
        ? 120
        : 240;
    if (
      !this.forceUpdate &&
      !zoomChanged &&
      now - this.lastUpdate < interval &&
      dir.dot(this.lastDirection) > 0.999 &&
      distanceDelta < minDistanceDelta
    ) {
      return;
    }
    this.forceUpdate = false;
    this.lastUpdate = now;
    this.lastDirection.copy(dir);
    this.lastDistance = distance;
    let tileSet = this.computeVisibleTiles(camera, size, zoom);
    while (tileSet.keys.length > this.maxTiles && zoom > this.provider.minZoom) {
      zoom -= 1;
      tileSet = this.computeVisibleTiles(camera, size, zoom);
    }
    if (zoom !== this.zoom) {
      this.zoom = zoom;
      this.clear();
    }
    const limitedKeys =
      tileSet.keys.length > this.maxTiles
        ? tileSet.keys.slice(0, this.maxTiles)
        : tileSet.keys;
    const desired = new Set(limitedKeys.map((entry) => entry.key));
    this.desiredKeys = desired;
    const queue = [];
    for (const entry of limitedKeys) {
      const cached = this.tiles.get(entry.key);
      if (cached?.mesh) {
        cached.lastUsed = now;
        continue;
      }
      if (this.pending.has(entry.key)) continue;
      queue.push(entry.tile);
    }
    this.queue = queue;
    this.drainQueue();
    for (const [key, tile] of this.tiles.entries()) {
      if (!tile.mesh) continue;
      const isDesired = desired.has(key);
      tile.mesh.visible = isDesired;
      tile.visible = isDesired;
      if (isDesired) {
        tile.lastUsed = now;
      }
    }
    this.evictCache(now);
  }

  evictCache(now) {
    if (this.tiles.size <= this.maxCache) return;
    const hidden = [];
    const visible = [];
    for (const [key, tile] of this.tiles.entries()) {
      const entry = {
        key,
        lastUsed: Number.isFinite(tile.lastUsed) ? tile.lastUsed : 0,
        tile,
      };
      if (tile.mesh?.visible) {
        visible.push(entry);
      } else {
        hidden.push(entry);
      }
    }
    hidden.sort((a, b) => a.lastUsed - b.lastUsed);
    visible.sort((a, b) => a.lastUsed - b.lastUsed);
    const candidates = hidden.concat(visible);
    for (const entry of candidates) {
      if (this.tiles.size <= this.maxCache) break;
      const tile = entry.tile;
      tile.mesh?.removeFromParent();
      tile.texture?.dispose();
      tile.geometry?.dispose();
      tile.material?.dispose();
      this.tiles.delete(entry.key);
    }
  }

  pickTileRadius(camera, zoom) {
    if (!camera?.isPerspectiveCamera) return 3;
    if (zoom <= 4) return 1;
    if (zoom <= 6) return 2;
    if (zoom <= 9) return 3;
    if (zoom <= 12) return 4;
    return 5;
  }

  pickFocusBoxPx(size, zoom) {
    const base = Math.min(size.width || 0, size.height || 0);
    if (!base) return 0;
    const maxRadius = Math.max(220, base * 0.32);
    const minRadius = Math.max(140, base * 0.2);
    const ratio = this.provider
      ? Math.max(0, Math.min(1, (zoom - this.provider.minZoom) / 6))
      : 0.5;
    return minRadius + (maxRadius - minRadius) * ratio;
  }

  computeVisibleTiles(camera, size, zoom) {
    const centerGeo = this.sampleGeo(camera, 0, 0);
    if (!centerGeo) {
      return { keys: [], tiles: new Map() };
    }
    const focusPx = this.pickFocusBoxPx(size, zoom);
    const ndcX = Math.min(0.95, (focusPx / Math.max(1, size.width)) * 2);
    const ndcY = Math.min(0.95, (focusPx / Math.max(1, size.height)) * 2);
    const focusSamples = [
      [0, 0],
      [ndcX, 0],
      [-ndcX, 0],
      [0, ndcY],
      [0, -ndcY],
      [ndcX, ndcY],
      [-ndcX, ndcY],
      [ndcX, -ndcY],
      [-ndcX, -ndcY],
    ];
    const focusGeos = focusSamples
      .map(([x, y]) => this.sampleGeo(camera, x, y))
      .filter(Boolean);
    const center = {
      x: tileXForLon(centerGeo.lon, zoom),
      y: tileYForLat(centerGeo.lat, zoom),
    };
    const n = 2 ** zoom;
    const tiles = new Map();
    const keys = [];
    if (focusGeos.length) {
      const latMin = Math.max(-85, Math.min(...focusGeos.map((g) => g.lat)));
      const latMax = Math.min(85, Math.max(...focusGeos.map((g) => g.lat)));
      const lonStats = this.computeLonRange(focusGeos.map((g) => g.lon));
      const lonSpan = lonStats.max - lonStats.min;
      const lonPadding = Math.max(1, lonSpan * 0.04);
      const lonMin = lonStats.min - lonPadding;
      const lonMax = lonStats.max + lonPadding;
      const yMin = Math.max(0, tileYForLat(latMax, zoom) - 1);
      const yMax = Math.min(n - 1, tileYForLat(latMin, zoom) + 1);
      let lonRanges = [];
      if (lonSpan >= 360) {
        lonRanges = [[-180, 180]];
      } else if (lonMin < -180) {
        lonRanges = [
          [lonMin + 360, 180],
          [-180, lonMax],
        ];
      } else if (lonMax > 180) {
        lonRanges = [
          [lonMin, 180],
          [-180, lonMax - 360],
        ];
      } else {
        lonRanges = [[lonMin, lonMax]];
      }
      const ranges = lonRanges.map(([startLon, endLon]) => [
        tileXForLon(startLon, zoom),
        tileXForLon(endLon, zoom),
      ]);
      const wrapX = (value) => ((value % n) + n) % n;
      ranges.forEach(([start, end]) => {
        for (let x = start - 1; x <= end + 1; x += 1) {
          const wrappedX = wrapX(x);
          for (let y = yMin; y <= yMax; y += 1) {
            const key = `${zoom}/${wrappedX}/${y}`;
            if (tiles.has(key)) continue;
            const bounds = tileBounds(wrappedX, y, zoom);
            const tile = { key, x: wrappedX, y, zoom, bounds };
            tiles.set(key, tile);
            const dist = (x - center.x) ** 2 + (y - center.y) ** 2;
            keys.push({ key, dist, tile });
          }
        }
      });
    } else {
      const radius = this.pickTileRadius(camera, zoom);
      for (let dx = -radius; dx <= radius; dx += 1) {
        const rawX = center.x + dx;
        const wrappedX = ((rawX % n) + n) % n;
        for (let dy = -radius; dy <= radius; dy += 1) {
          const y = center.y + dy;
          if (y < 0 || y >= n) continue;
          const key = `${zoom}/${wrappedX}/${y}`;
          if (tiles.has(key)) continue;
          const bounds = tileBounds(wrappedX, y, zoom);
          const tile = { key, x: wrappedX, y, zoom, bounds };
          tiles.set(key, tile);
          const dist = dx * dx + dy * dy;
          keys.push({ key, dist, tile });
        }
      }
    }
    keys.sort((a, b) => a.dist - b.dist);
    return { keys, tiles };
  }

  computeLonRange(lons) {
    let sumSin = 0;
    let sumCos = 0;
    lons.forEach((lon) => {
      const rad = (lon * Math.PI) / 180;
      sumSin += Math.sin(rad);
      sumCos += Math.cos(rad);
    });
    const mean = (Math.atan2(sumSin, sumCos) * 180) / Math.PI;
    let min = 180;
    let max = -180;
    lons.forEach((lon) => {
      let delta = lon - mean;
      delta = ((delta + 540) % 360) - 180;
      min = Math.min(min, delta);
      max = Math.max(max, delta);
    });
    return { min: mean + min, max: mean + max };
  }

  sampleHorizon(camera) {
    if (!camera?.isPerspectiveCamera) return [];
    const distance = camera.position.length();
    if (!Number.isFinite(distance) || distance <= this.radius) return [];
    const horizonAngle = Math.acos(this.radius / distance);
    const fovV = THREE.MathUtils.degToRad(camera.fov);
    const fovH = 2 * Math.atan(Math.tan(fovV / 2) * camera.aspect);
    const cornerAngle = Math.atan(
      Math.hypot(Math.tan(fovV / 2), Math.tan(fovH / 2)),
    );
    const angle = Math.min(horizonAngle, cornerAngle * 1.15);
    const center = camera.position.clone().normalize();
    const up = camera.up.clone().normalize();
    const right = new THREE.Vector3().crossVectors(up, center).normalize();
    const upOrtho = new THREE.Vector3().crossVectors(center, right).normalize();
    const geos = [];
    const segments = 18;
    for (let i = 0; i < segments; i += 1) {
      const t = (i / segments) * TWO_PI;
      const dir = new THREE.Vector3()
        .copy(center)
        .multiplyScalar(Math.cos(angle))
        .addScaledVector(right, Math.sin(angle) * Math.cos(t))
        .addScaledVector(upOrtho, Math.sin(angle) * Math.sin(t));
      if (this.rotationY) {
        dir.applyAxisAngle(AXIS_Y, -this.rotationY);
      }
      geos.push(sphereToGeoTile(dir));
    }
    return geos;
  }

  sampleGeo(camera, ndcX, ndcY) {
    const dir = this.tmpVec.set(ndcX, ndcY, 0.5).unproject(camera).sub(camera.position);
    dir.normalize();
    this.ray.origin.copy(camera.position);
    this.ray.direction.copy(dir);
    const t = this.raySphereIntersect(this.ray, this.radius);
    if (!Number.isFinite(t)) return null;
    this.tmpPoint.copy(this.ray.direction).multiplyScalar(t).add(this.ray.origin);
    if (this.rotationY) {
      this.tmpPoint.applyAxisAngle(AXIS_Y, -this.rotationY);
    }
    return sphereToGeoTile(this.tmpPoint);
  }

  raySphereIntersect(ray, radius) {
    const origin = ray.origin;
    const dir = ray.direction;
    const b = 2 * origin.dot(dir);
    const c = origin.dot(origin) - radius * radius;
    const disc = b * b - 4 * c;
    if (disc < 0) return null;
    const t1 = (-b - Math.sqrt(disc)) / 2;
    const t2 = (-b + Math.sqrt(disc)) / 2;
    if (t1 > 0) return t1;
    if (t2 > 0) return t2;
    return null;
  }

  drainQueue() {
    while (this.inFlight < this.maxInFlight && this.queue.length) {
      const tile = this.queue.shift();
      if (!tile) break;
      this.loadTile(tile);
    }
  }

  buildUrl(tile) {
    if (!this.provider) return "";
    let url = this.provider.url
      .replace("{z}", tile.zoom)
      .replace("{x}", tile.x)
      .replace("{y}", tile.y);
    if (this.provider.params) {
      Object.entries(this.provider.params).forEach(([key, value]) => {
        url = url.replace(`{${key}}`, encodeURIComponent(String(value)));
      });
    }
    return url;
  }

  loadTile(tile) {
    if (!this.provider) return;
    const url = this.buildUrl(tile);
    this.pending.add(tile.key);
    this.inFlight += 1;
    this.loader.load(
      url,
      (texture) => {
        const now = performance.now();
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.flipY = false;
        texture.generateMipmaps = false;
        texture.minFilter = THREE.LinearFilter;
        texture.magFilter = THREE.LinearFilter;
        texture.anisotropy = this.renderer?.capabilities?.getMaxAnisotropy?.() || 1;
        texture.needsUpdate = true;
        const geometry = this.buildTileGeometry(tile.bounds);
        const opacity = Number.isFinite(this.provider.opacity)
          ? this.provider.opacity
          : 1.0;
        const material = new THREE.MeshBasicMaterial({
          map: texture,
          transparent: opacity < 0.999 || this.provider.transparent === true,
          opacity,
          color: new THREE.Color(0xffffff),
          side: THREE.FrontSide,
        });
        material.depthTest = this.provider.depthTest ?? true;
        material.depthWrite =
          this.provider.depthWrite ?? (!material.transparent && material.depthTest);
        if (Number.isFinite(this.provider.alphaTest)) {
          material.alphaTest = this.provider.alphaTest;
        }
        const offsetFactor = Number.isFinite(this.provider.polygonOffsetFactor)
          ? this.provider.polygonOffsetFactor
          : -3;
        const offsetUnits = Number.isFinite(this.provider.polygonOffsetUnits)
          ? this.provider.polygonOffsetUnits
          : -3;
        material.polygonOffset = offsetFactor !== 0 || offsetUnits !== 0;
        material.polygonOffsetFactor = offsetFactor;
        material.polygonOffsetUnits = offsetUnits;
        const mesh = new THREE.Mesh(geometry, material);
        mesh.renderOrder = this.provider.renderOrder ?? 10;
        geometry.computeBoundingSphere();
        mesh.frustumCulled = true;
        const visible = this.desiredKeys?.has(tile.key);
        mesh.visible = Boolean(visible);
        this.group.add(mesh);
        this.tiles.set(tile.key, {
          mesh,
          texture,
          geometry,
          material,
          visible: Boolean(visible),
          lastUsed: now,
        });
        this.pending.delete(tile.key);
        this.inFlight = Math.max(0, this.inFlight - 1);
        this.drainQueue();
      },
      undefined,
      () => {
        this.pending.delete(tile.key);
        this.inFlight = Math.max(0, this.inFlight - 1);
        this.drainQueue();
      },
    );
  }

  buildTileGeometry(bounds) {
    const latNorth = bounds.latNorth;
    const latSouth = bounds.latSouth;
    const lonWest = bounds.lonWest;
    const lonEast = bounds.lonEast;
    const lonSpan = Math.abs(lonEast - lonWest);
    const latSpan = Math.abs(latNorth - latSouth);
    const widthSegments = Math.min(128, Math.max(12, Math.round(lonSpan / 2)));
    const heightSegments = Math.min(96, Math.max(10, Math.round(latSpan / 2)));
    const phiStart = ((lonWest + 180) * Math.PI) / 180;
    const phiLength = ((lonEast - lonWest) * Math.PI) / 180;
    const thetaStart = ((90 - latNorth) * Math.PI) / 180;
    const thetaLength = ((latNorth - latSouth) * Math.PI) / 180;
    const geometry = new THREE.SphereGeometry(
      this.radius,
      widthSegments,
      heightSegments,
      phiStart,
      phiLength,
      thetaStart,
      thetaLength,
    );
    const baseUv = geometry.getAttribute("uv");
    const uv = new Float32Array(baseUv.count * 2);
    const xWest = (lonWest + 180) / 360;
    const xEast = (lonEast + 180) / 360;
    const yNorth = mercatorYForLat(latNorth);
    const ySouth = mercatorYForLat(latSouth);
    const xSpan = xEast - xWest;
    const ySpan = ySouth - yNorth;
    for (let i = 0; i < baseUv.count; i += 1) {
      const uBase = baseUv.getX(i);
      const vBase = baseUv.getY(i);
      const phi = phiStart + uBase * phiLength;
      const theta = thetaStart + (1 - vBase) * thetaLength;
      const lat = 90 - (theta * 180) / Math.PI;
      const xNorm = phi / TWO_PI;
      const yNorm = mercatorYForLat(lat);
      let u = (xNorm - xWest) / xSpan;
      let v = (yNorm - yNorth) / ySpan;
      u = Math.min(1, Math.max(0, u));
      v = Math.min(1, Math.max(0, v));
      uv[i * 2] = u;
      uv[i * 2 + 1] = v;
    }
    geometry.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
    return geometry;
  }
}

class Renderer3D {
  constructor(canvas) {
    this.canvas = canvas;
    this.renderer = null;
    this.scene = null;
    this.camera = null;
    this.cameraPerspective = null;
    this.cameraIso = null;
    this.controls = null;
    this.instances = null;
    this.globe = null;
    this.atmosphere = null;
    this.mapPlane = null;
    this.globeRadius = 120;
    this.mode = "globe";
    this.size = { width: 1, height: 1 };
    this.planeSize = { width: this.globeRadius * 4, height: this.globeRadius * 2 };
    this.isoFrustum = this.planeSize.height * 1.4;
    this.markerAltitude = 3.0;
    this.clouds = null;
    this.axisHelper = null;
    this.gridLines = null;
    this.tileManager = null;
    this.tileProvider = null;
    this.tileZoom = null;
    this.weatherManager = null;
    this.weatherProvider = null;
    this.weatherField = WEATHER_CONFIG.defaultField;
    this.weatherTime = WEATHER_CONFIG.defaultTime;
    this.weatherFormat = WEATHER_CONFIG.defaultFormat;
    this.weatherOpacity = WEATHER_CONFIG.defaultOpacity;
    this.weatherVisible = false;
    this.globeRotation = Math.PI;
    this.dayMap = null;
    this.nightMap = null;
    this.normalMap = null;
    this.specularMap = null;
    this.cloudsMap = null;
    this.globeMaterial = null;
    this.lightingMode = "day";
    this.showClouds = false;
    this.showAxes = true;
    this.showGrid = true;
    this.baseRotateSpeed = 0.85;
    this.crosshairRadius = 18;
    this.crosshairInputRadius = 120;
    this.crosshairDeadzone = 0.1;
    this.crosshairActive = false;
    this.crosshairPointerId = null;
    this.crosshairCenter = { x: 0, y: 0 };
    this.crosshairLast = { x: 0, y: 0 };
    this.crosshairVector = new THREE.Vector2();
    this.crosshairHandlers = null;
    this.trails = [];
    this.lastCameraVec = null;
    this.lastTrailAt = 0;
    this.defaultDistance = this.globeRadius * 2.6;
    this.fillRatio = 0.72;
    this.focusTween = null;
    this.tmp = new THREE.Object3D();
    this.tmpVec = new THREE.Vector3();
    this.tmpVec2 = new THREE.Vector3();
    this.tmpVec3 = new THREE.Vector3();
    this.tmpAxis = new THREE.Vector3();
    this.tmpQuat = new THREE.Quaternion();
    this.tmpSpherical = new THREE.Spherical();
  }

  init() {
    if (!this.canvas) return;
    this.renderer = new THREE.WebGLRenderer({
      canvas: this.canvas,
      antialias: true,
      alpha: true,
    });
    this.renderer.setPixelRatio(devicePixelRatio);
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.05;
    this.scene = new THREE.Scene();
    this.cameraPerspective = new THREE.PerspectiveCamera(55, 1, 0.1, 6000);
    this.cameraPerspective.position.set(0, 0, this.globeRadius * 2.8);
    this.cameraIso = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 4000);
    this.cameraIso.position.set(
      this.globeRadius * 1.6,
      this.globeRadius * 1.6,
      this.globeRadius * 1.6,
    );
    this.cameraIso.up.set(0, 1, 0);
    this.cameraIso.lookAt(0, 0, 0);
    this.camera = this.cameraPerspective;

    const hemi = new THREE.HemisphereLight(0xffffff, 0x3f3f3f, 0.65);
    this.scene.add(hemi);
    const ambient = new THREE.AmbientLight(0x0f172a, 0.25);
    this.scene.add(ambient);
    const sun = new THREE.DirectionalLight(0xffffff, 1.15);
    sun.position.set(280, 160, 120);
    this.scene.add(sun);
    const rim = new THREE.DirectionalLight(0x93c5fd, 0.35);
    rim.position.set(-240, -180, -220);
    this.scene.add(rim);

    const loader = new THREE.TextureLoader();
    this.dayMap = loader.load("/static/maps/8k_earth_daymap.png");
    this.nightMap = loader.load("/static/maps/8k_earth_nightmap.png");
    this.normalMap = loader.load("/static/maps/8k_earth_normal_map.jpg");
    this.specularMap = loader.load("/static/maps/8k_earth_specular_map.jpg");
    this.cloudsMap = loader.load("/static/maps/8k_earth_clouds.png");

    this.dayMap.colorSpace = THREE.SRGBColorSpace;
    this.nightMap.colorSpace = THREE.SRGBColorSpace;
    this.cloudsMap.colorSpace = THREE.SRGBColorSpace;
    this.normalMap.colorSpace = THREE.NoColorSpace;
    this.specularMap.colorSpace = THREE.NoColorSpace;

    const anisotropy = this.renderer.capabilities.getMaxAnisotropy();
    this.dayMap.anisotropy = anisotropy;
    this.nightMap.anisotropy = anisotropy;
    this.normalMap.anisotropy = anisotropy;
    this.specularMap.anisotropy = anisotropy;
    this.cloudsMap.anisotropy = anisotropy;

    this.globeMaterial = new THREE.MeshPhongMaterial({
      map: this.dayMap,
      color: new THREE.Color(0xffffff),
      side: THREE.FrontSide,
      normalMap: this.normalMap,
      normalScale: new THREE.Vector2(1.1, 1.1),
      specularMap: this.specularMap,
      specular: new THREE.Color(0x666666),
      shininess: 28,
      emissive: new THREE.Color(0x0b0f23),
      emissiveMap: this.nightMap,
      emissiveIntensity: 0.35,
    });
    this.globe = new THREE.Mesh(
      new THREE.SphereGeometry(this.globeRadius, 128, 128),
      this.globeMaterial,
    );
    this.globe.rotation.y = this.globeRotation;
    this.scene.add(this.globe);

    this.atmosphere = new THREE.Mesh(
      new THREE.SphereGeometry(this.globeRadius + 2, 64, 64),
      new THREE.MeshBasicMaterial({
        color: 0x7dd3fc,
        transparent: true,
        opacity: 0.08,
        side: THREE.FrontSide,
      }),
    );
    this.atmosphere.renderOrder = 4;
    this.scene.add(this.atmosphere);

    this.clouds = new THREE.Mesh(
      new THREE.SphereGeometry(this.globeRadius + 2.2, 128, 128),
      new THREE.MeshPhongMaterial({
        map: this.cloudsMap,
        alphaMap: this.cloudsMap,
        transparent: true,
        opacity: 0.9,
        alphaTest: 0.02,
        depthWrite: false,
        blending: THREE.AdditiveBlending,
        color: new THREE.Color(0xffffff),
        side: THREE.FrontSide,
      }),
    );
    this.clouds.material.depthTest = true;
    this.clouds.renderOrder = 3;
    this.clouds.rotation.y = this.globeRotation;
    this.scene.add(this.clouds);

    this.tileManager = new TileManager(
      this.scene,
      this.globeRadius + 0.08,
      this.renderer,
      this.globeRotation,
    );
    this.tileManager.setBaseDistance(this.defaultDistance);

    this.weatherManager = new TileManager(
      this.scene,
      this.globeRadius + 0.1,
      this.renderer,
      this.globeRotation,
    );
    this.weatherManager.maxTiles = Math.min(WEATHER_CONFIG.maxTiles, 120);
    this.weatherManager.maxCache = Math.max(256, this.weatherManager.maxTiles * 3);
    this.weatherManager.maxInFlight = WEATHER_CONFIG.maxInFlight;
    this.weatherManager.setBaseDistance(this.defaultDistance);

    const planeMaterial = new THREE.MeshStandardMaterial({
      map: this.dayMap,
      roughness: 0.9,
      metalness: 0.0,
      side: THREE.FrontSide,
    });
    this.mapPlane = new THREE.Mesh(
      new THREE.PlaneGeometry(this.planeSize.width, this.planeSize.height, 1, 1),
      planeMaterial,
    );
    this.mapPlane.rotation.x = -Math.PI / 2;
    this.mapPlane.position.y = -1;
    this.mapPlane.visible = false;
    this.scene.add(this.mapPlane);

    this.axisHelper = new THREE.AxesHelper(this.globeRadius * 1.6);
    this.axisHelper.visible = true;
    this.axisHelper.setColors(0xff0000, 0x00ff00, 0x0000ff);
    this.axisHelper.renderOrder = 10020;
    this.axisHelper.traverse((child) => {
      if (!child || !child.material) return;
      child.renderOrder = 10020;
      const materials = Array.isArray(child.material) ? child.material : [child.material];
      materials.forEach((material) => {
        if (!material) return;
        material.depthTest = false;
        material.depthWrite = false;
      });
    });
    this.scene.add(this.axisHelper);

    this.gridLines = this.buildLatLonGrid(this.globeRadius + 0.6, 20, 10);
    this.gridLines.renderOrder = 10010;
    this.scene.add(this.gridLines);

    this.setLightingMode("day");
    this.setCloudsVisible(false);
    this.setAxesVisible(true);
    this.setGridVisible(true);
    this.setTileProvider(TILE_CONFIG.activeProvider);
    this.refreshWeatherProvider();
    this.setWeatherVisible(false);
    this.setMode("globe", true);
  }

  attachControls(allowRotate) {
    if (!this.canvas || !this.camera) return;
    this.controls?.dispose();
    this.controls = new OrbitControls(this.camera, this.canvas);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.08;
    this.controls.enablePan = false;
    this.controls.enableRotate = allowRotate;
    this.controls.target.set(0, 0, 0);
    this.controls.screenSpacePanning = true;
    if (this.camera.isPerspectiveCamera) {
      this.controls.minDistance = this.globeRadius * 1.05;
      this.controls.maxDistance = this.globeRadius * 6;
    }
    if (this.camera.isOrthographicCamera) {
      this.controls.minZoom = 0.6;
      this.controls.maxZoom = 2.4;
    }
    this.lastCameraVec = this.camera.position.clone().normalize();
    this.controls.addEventListener("change", () => {
      this.recordCameraTrail();
      this.tileManager?.markDirty();
      this.weatherManager?.markDirty();
    });
    this.attachCrosshairControls(allowRotate);
  }

  attachCrosshairControls(allowRotate) {
    if (!this.canvas) return;
    this.detachCrosshairControls();
    const onPointerDown = (event) => {
      if (!allowRotate || event.button !== 0) return;
      const rect = this.canvas.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const dx = event.clientX - cx;
      const dy = event.clientY - cy;
      if (Math.hypot(dx, dy) > this.crosshairRadius) return;
      this.crosshairActive = true;
      this.crosshairPointerId = event.pointerId;
      this.crosshairCenter = { x: cx, y: cy };
      this.updateCrosshairInput(event.clientX, event.clientY);
      if (this.controls) this.controls.enabled = false;
      if (this.canvas.setPointerCapture) {
        this.canvas.setPointerCapture(event.pointerId);
      }
      event.preventDefault();
      event.stopPropagation();
    };
    const onPointerMove = (event) => {
      if (!this.crosshairActive || event.pointerId !== this.crosshairPointerId) return;
      this.updateCrosshairInput(event.clientX, event.clientY);
      event.preventDefault();
      event.stopPropagation();
    };
    const onPointerUp = (event) => {
      if (event.pointerId !== this.crosshairPointerId) return;
      this.crosshairActive = false;
      this.crosshairPointerId = null;
      this.crosshairVector.set(0, 0);
      if (this.controls) this.controls.enabled = true;
      if (this.canvas.releasePointerCapture) {
        this.canvas.releasePointerCapture(event.pointerId);
      }
    };
    this.crosshairHandlers = { onPointerDown, onPointerMove, onPointerUp };
    this.canvas.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    window.addEventListener("pointercancel", onPointerUp);
  }

  updateCrosshairInput(clientX, clientY) {
    const dx = clientX - this.crosshairCenter.x;
    const dy = clientY - this.crosshairCenter.y;
    const radius = Math.max(24, this.crosshairInputRadius);
    const len = Math.hypot(dx, dy);
    if (len < 0.001) {
      this.crosshairVector.set(0, 0);
      return;
    }
    const scale = len > radius ? radius / len : 1;
    this.crosshairVector.set((dx * scale) / radius, (dy * scale) / radius);
  }

  detachCrosshairControls() {
    if (!this.crosshairHandlers || !this.canvas) return;
    const { onPointerDown, onPointerMove, onPointerUp } = this.crosshairHandlers;
    this.canvas.removeEventListener("pointerdown", onPointerDown);
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    window.removeEventListener("pointercancel", onPointerUp);
    this.crosshairHandlers = null;
  }

  updateCrosshairDrive(deltaMs) {
    if (!this.crosshairActive || !this.camera || !this.controls) return;
    const mag = this.crosshairVector.length();
    if (mag <= this.crosshairDeadzone) return;
    const strength = Math.min(1, (mag - this.crosshairDeadzone) / (1 - this.crosshairDeadzone));
    const rotateSpeed = this.getRotateSpeed();
    const rate = rotateSpeed * 2.4;
    const scale = (deltaMs / 1000) * rate * strength;
    const thetaDelta = this.crosshairVector.x * scale;
    const phiDelta = this.crosshairVector.y * scale;
    this.applySphericalDelta(thetaDelta, phiDelta);
  }

  getRotateSpeed() {
    if (!this.camera?.isPerspectiveCamera) return this.baseRotateSpeed;
    const distance = this.camera.position.length();
    const ratio = this.defaultDistance ? distance / this.defaultDistance : 1;
    const eased = Math.max(0.02, Math.min(1.1, ratio));
    return this.baseRotateSpeed * eased * eased * eased;
  }

  rotateFromCrosshair(dx, dy) {
    if (!this.camera || !this.controls) return;
    const target = this.controls.target || new THREE.Vector3();
    const rotateSpeed = this.getRotateSpeed();
    const scale = (2 * Math.PI * rotateSpeed) / Math.max(1, this.size.height);
    const thetaDelta = -dx * scale;
    const phiDelta = dy * scale;
    this.applySphericalDelta(thetaDelta, phiDelta);
  }

  applySphericalDelta(thetaDelta, phiDelta) {
    const target = this.controls?.target || new THREE.Vector3();
    this.tmpVec.copy(this.camera.position).sub(target);
    this.tmpSpherical.setFromVector3(this.tmpVec);
    this.tmpSpherical.theta += thetaDelta;
    this.tmpSpherical.phi += phiDelta;
    const EPS = 1e-5;
    this.tmpSpherical.phi = Math.max(EPS, Math.min(Math.PI - EPS, this.tmpSpherical.phi));
    this.tmpVec.setFromSpherical(this.tmpSpherical).add(target);
    this.camera.position.copy(this.tmpVec);
    this.camera.lookAt(target);
    this.controls?.update();
    this.recordCameraTrail();
  }

  setMode(mode, skipResize = false) {
    if (mode !== "globe" && mode !== "iso") return;
    this.mode = mode;
    this.camera = mode === "iso" ? this.cameraIso : this.cameraPerspective;
    if (this.camera === this.cameraIso) {
      this.camera.position.set(
        this.globeRadius * 1.6,
        this.globeRadius * 1.6,
        this.globeRadius * 1.6,
      );
      this.camera.lookAt(0, 0, 0);
    } else if (this.camera === this.cameraPerspective) {
      this.camera.position.set(
        0,
        this.globeRadius * 1.5,
        this.globeRadius * 2.7,
      );
    }
    if (this.globe) this.globe.visible = mode === "globe";
    if (this.atmosphere) this.atmosphere.visible = mode === "globe";
    if (this.mapPlane) this.mapPlane.visible = mode === "iso";
    if (this.clouds) this.clouds.visible = mode === "globe" && this.showClouds;
    if (this.axisHelper) this.axisHelper.visible = mode === "globe" && this.showAxes;
    if (this.gridLines) this.gridLines.visible = mode === "globe" && this.showGrid;
    if (this.tileManager) {
      this.tileManager.group.visible = mode === "globe" && Boolean(this.tileProvider);
    }
    if (this.weatherManager) {
      this.weatherManager.group.visible =
        mode === "globe" && this.weatherVisible && Boolean(this.weatherProvider);
    }
    if (els.map2d) {
      els.map2d.style.display = mode === "iso" ? "block" : "none";
    }
    this.attachControls(mode === "globe");
    if (!skipResize) this.resize(this.size.width, this.size.height);
  }

  resize(width, height) {
    if (!this.renderer || !this.camera) return;
    this.size = { width, height };
    this.renderer.setSize(width, height, false);
    if (this.cameraPerspective) {
      this.cameraPerspective.aspect = width / height;
      this.cameraPerspective.updateProjectionMatrix();
    }
    if (this.cameraIso) {
      const aspect = width / height;
      const frustum = this.isoFrustum;
      this.cameraIso.left = (-frustum * aspect) / 2;
      this.cameraIso.right = (frustum * aspect) / 2;
      this.cameraIso.top = frustum / 2;
      this.cameraIso.bottom = -frustum / 2;
      this.cameraIso.updateProjectionMatrix();
    }
    this.updateCameraDistance(width, height);
  }

  positionForGeo(geo, altitude) {
    if (this.mode === "iso") {
      const plane = geoToPlane(geo, this.planeSize);
      return { x: plane.x, y: altitude, z: plane.z };
    }
    return geoToSphere(geo, this.globeRadius + altitude);
  }

  projectToScreen(point) {
    if (!this.camera) return null;
    let behind = false;
    this.tmpVec.set(point.x, point.y, point.z);
    if (this.mode === "globe") {
      this.tmpVec2.set(point.x, point.y, point.z).normalize();
      this.tmpVec3.copy(this.camera.position).normalize();
      behind = this.tmpVec2.dot(this.tmpVec3) <= 0;
    }
    this.tmpVec.project(this.camera);
    const x = (this.tmpVec.x * 0.5 + 0.5) * this.size.width;
    const y = (-this.tmpVec.y * 0.5 + 0.5) * this.size.height;
    const inFrustum =
      this.tmpVec.z >= -1 &&
      this.tmpVec.z <= 1 &&
      x >= 0 &&
      x <= this.size.width &&
      y >= 0 &&
      y <= this.size.height;
    const visible = !behind && inFrustum;
    return { x, y, visible, behind };
  }

  geoFromScreen(x, y) {
    if (!this.camera || !this.tileManager || !this.size.width || !this.size.height) {
      return null;
    }
    const ndcX = (x / this.size.width) * 2 - 1;
    const ndcY = -(y / this.size.height) * 2 + 1;
    return this.tileManager.sampleGeo(this.camera, ndcX, ndcY);
  }

  setInstances(points) {
    if (!this.instances) return;
    this.instances.count = points.length;
    const color = new THREE.Color();
    points.forEach((point, index) => {
      this.tmp.position.set(point.x, point.y, point.z);
      this.tmp.updateMatrix();
      this.instances.setMatrixAt(index, this.tmp.matrix);
      color.set(point.color || "#ffffff");
      this.instances.setColorAt(index, color);
    });
    this.instances.instanceMatrix.needsUpdate = true;
    if (this.instances.instanceColor) {
      this.instances.instanceColor.needsUpdate = true;
    }
  }

  render(deltaMs = 16) {
    if (!this.renderer || !this.scene || !this.camera) return;
    if (els.map3d && els.map3d.style.display === "none") return;
    if (this.clouds && this.mode === "globe" && this.showClouds) {
      this.clouds.rotation.y += 0.00025;
    }
    this.updateTrails();
    this.updateFocus();
    this.updateRotateSpeed();
    this.updateCrosshairDrive(deltaMs);
    this.controls?.update();
    if (this.tileManager && this.tileProvider && this.mode === "globe") {
      this.tileManager.update(this.camera, this.size);
      this.tileZoom = this.tileManager.zoom;
    }
    if (this.weatherManager && this.weatherProvider && this.weatherVisible && this.mode === "globe") {
      this.weatherManager.update(this.camera, this.size);
    }
    this.renderer.render(this.scene, this.camera);
  }

  updateCameraDistance(width, height) {
    if (!this.cameraPerspective) return;
    if (!width || !height) return;
    const fovV = THREE.MathUtils.degToRad(this.cameraPerspective.fov);
    const aspect = width / height;
    const fovH = 2 * Math.atan(Math.tan(fovV / 2) * aspect);
    const desiredRadiusPx = width * this.fillRatio * 0.5;
    const widthDistance =
      (width * this.globeRadius) /
      (desiredRadiusPx * Math.tan(fovH / 2) * 2);
    const maxHeightRadiusPx = height * 0.48;
    const heightDistance =
      (height * this.globeRadius) /
      (maxHeightRadiusPx * Math.tan(fovV / 2) * 2);
    const distance = Math.max(widthDistance, heightDistance);
    if (!Number.isFinite(distance) || distance <= 0) return;
    this.defaultDistance = distance;
    this.cameraPerspective.position.setLength(distance);
    if (this.controls?.object?.isPerspectiveCamera) {
      const minDistance = Math.max(this.globeRadius * 1.02, distance * 0.25);
      this.controls.minDistance = minDistance;
      this.controls.maxDistance = distance * 3;
    }
    if (this.tileManager) {
      this.tileManager.setBaseDistance(distance);
    }
    if (this.weatherManager) {
      this.weatherManager.setBaseDistance(distance);
    }
  }

  updateRotateSpeed() {
    if (!this.controls || !this.camera) return;
    if (!this.camera.isPerspectiveCamera) return;
    this.controls.rotateSpeed = this.getRotateSpeed();
  }

  setLightingMode(mode) {
    if (!this.globeMaterial) return;
    this.lightingMode = mode === "night" ? "night" : "day";
    if (this.lightingMode === "night") {
      this.globeMaterial.map = this.nightMap;
      this.globeMaterial.emissiveMap = null;
      this.globeMaterial.emissiveIntensity = 0;
      this.globeMaterial.specular.setHex(0x222222);
      this.globeMaterial.shininess = 8;
      this.globeMaterial.color.setHex(0xd1d5db);
    } else {
      this.globeMaterial.map = this.dayMap;
      this.globeMaterial.emissiveMap = this.nightMap;
      this.globeMaterial.emissiveIntensity = 0.35;
      this.globeMaterial.specular.setHex(0x666666);
      this.globeMaterial.shininess = 28;
      this.globeMaterial.color.setHex(0xffffff);
    }
    this.globeMaterial.needsUpdate = true;
  }

  setCloudsVisible(visible) {
    this.showClouds = Boolean(visible);
    if (this.clouds) {
      this.clouds.visible = this.showClouds && this.mode === "globe";
    }
  }

  setAxesVisible(visible) {
    this.showAxes = Boolean(visible);
    if (this.axisHelper) {
      this.axisHelper.visible = this.showAxes && this.mode === "globe";
    }
  }

  setGridVisible(visible) {
    this.showGrid = Boolean(visible);
    if (this.gridLines) {
      this.gridLines.visible = this.showGrid && this.mode === "globe";
    }
  }

  setTileProvider(providerId) {
    const provider = TILE_CONFIG.providers[providerId] || null;
    this.tileProvider = provider;
    if (this.tileManager) {
      this.tileManager.setProvider(provider);
      this.tileZoom = provider ? this.tileManager.zoom : null;
    }
  }

  buildWeatherProvider() {
    if (!WEATHER_CONFIG.enabled) return null;
    return {
      id: "weather",
      name: "Weather Overlay",
      url: "/ui/tiles/weather/{z}/{x}/{y}?field={field}&time={time}&format={format}",
      minZoom: WEATHER_CONFIG.minZoom,
      maxZoom: WEATHER_CONFIG.maxZoom,
      opacity: this.weatherOpacity,
      renderOrder: 50,
      depthTest: false,
      depthWrite: false,
      polygonOffsetFactor: -6,
      polygonOffsetUnits: -6,
      updateIntervalMs: WEATHER_CONFIG.updateIntervalMs,
      params: {
        field: this.weatherField,
        time: this.weatherTime,
        format: this.weatherFormat,
      },
    };
  }

  refreshWeatherProvider() {
    const provider = this.buildWeatherProvider();
    this.weatherProvider = provider;
    if (this.weatherManager) {
      this.weatherManager.setProvider(provider);
      if (WEATHER_CONFIG.maxTiles) {
        this.weatherManager.maxTiles = Math.min(WEATHER_CONFIG.maxTiles, 120);
        this.weatherManager.maxCache = Math.max(256, this.weatherManager.maxTiles * 3);
      }
      this.weatherManager.maxInFlight = WEATHER_CONFIG.maxInFlight;
      this.weatherManager.group.visible =
        this.mode === "globe" && this.weatherVisible && Boolean(provider);
    }
  }

  setWeatherVisible(visible) {
    this.weatherVisible = Boolean(visible);
    if (this.weatherManager) {
      this.weatherManager.group.visible =
        this.mode === "globe" && this.weatherVisible && Boolean(this.weatherProvider);
      if (this.weatherVisible) {
        this.weatherManager.markDirty();
      }
    }
  }

  setWeatherField(field) {
    if (!field || field === this.weatherField) return;
    this.weatherField = field;
    this.refreshWeatherProvider();
    if (this.weatherManager) {
      this.weatherManager.markDirty();
    }
  }

  setWeatherTime(time) {
    if (!time || time === this.weatherTime) return;
    this.weatherTime = time;
    this.refreshWeatherProvider();
    if (this.weatherManager) {
      this.weatherManager.markDirty();
    }
  }

  buildLatLonGrid(radius, lonStep, latStep) {
    const vertices = [];
    const toRad = THREE.MathUtils.degToRad;
    const addLine = (points) => {
      for (let i = 0; i < points.length - 1; i += 1) {
        const a = points[i];
        const b = points[i + 1];
        vertices.push(a.x, a.y, a.z, b.x, b.y, b.z);
      }
    };

    for (let lon = -180; lon <= 180; lon += lonStep) {
      const points = [];
      for (let lat = -90; lat <= 90; lat += latStep) {
        const phi = toRad(90 - lat);
        const theta = toRad(lon + 180);
        points.push(
          new THREE.Vector3(
            radius * Math.sin(phi) * Math.cos(theta),
            radius * Math.cos(phi),
            radius * Math.sin(phi) * Math.sin(theta),
          ),
        );
      }
      addLine(points);
    }

    for (let lat = -60; lat <= 60; lat += lonStep) {
      const points = [];
      for (let lon = -180; lon <= 180; lon += latStep) {
        const phi = toRad(90 - lat);
        const theta = toRad(lon + 180);
        points.push(
          new THREE.Vector3(
            radius * Math.sin(phi) * Math.cos(theta),
            radius * Math.cos(phi),
            radius * Math.sin(phi) * Math.sin(theta),
          ),
        );
      }
      addLine(points);
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.Float32BufferAttribute(vertices, 3));
    const material = new THREE.LineBasicMaterial({
      color: 0xf97316,
      transparent: true,
      opacity: 0.45,
    });
    material.depthTest = true;
    material.depthWrite = false;
    const line = new THREE.LineSegments(geometry, material);
    line.renderOrder = 10010;
    return line;
  }

  recordCameraTrail() {
    if (!this.camera || this.mode !== "globe") return;
    const now = performance.now();
    if (now - this.lastTrailAt < 120) return;
    this.lastTrailAt = now;
    const current = this.camera.position.clone().normalize();
    if (!this.lastCameraVec) {
      this.lastCameraVec = current;
      return;
    }
    const angle = this.lastCameraVec.angleTo(current);
    if (angle < 0.02) return;
    const line = this.createArcLine(this.lastCameraVec, current);
    this.scene.add(line);
    this.trails.push({ line, createdAt: now, duration: 2600 });
    this.lastCameraVec.copy(current);
  }

  createArcLine(startVec, endVec) {
    const points = [];
    const segments = 32;
    const radius = this.globeRadius + 1.4;
    const start = startVec.clone().normalize();
    const end = endVec.clone().normalize();
    const angle = start.angleTo(end);
    if (angle < 0.0001) {
      points.push(start.clone().multiplyScalar(radius), end.clone().multiplyScalar(radius));
    } else {
      this.tmpAxis.crossVectors(start, end);
      if (this.tmpAxis.lengthSq() < 1e-6) {
        this.tmpAxis.set(0, 1, 0);
      } else {
        this.tmpAxis.normalize();
      }
      for (let i = 0; i <= segments; i += 1) {
        const t = i / segments;
        this.tmpQuat.setFromAxisAngle(this.tmpAxis, angle * t);
        const point = start.clone().applyQuaternion(this.tmpQuat).multiplyScalar(radius);
        points.push(point);
      }
    }
    const geometry = new THREE.BufferGeometry().setFromPoints(points);
    const material = new THREE.LineBasicMaterial({
      color: 0xef4444,
      transparent: true,
      opacity: 0.9,
    });
    return new THREE.Line(geometry, material);
  }

  updateTrails() {
    if (!this.trails.length) return;
    const now = performance.now();
    this.trails = this.trails.filter((trail) => {
      const elapsed = now - trail.createdAt;
      const remaining = Math.max(0, trail.duration - elapsed);
      const alpha = remaining / trail.duration;
      trail.line.material.opacity = alpha;
      if (remaining <= 0) {
        this.scene.remove(trail.line);
        trail.line.geometry.dispose();
        return false;
      }
      return true;
    });
  }

  focusOnGeo(geo) {
    if (!this.camera || !this.controls) return;
    const target = geoToSphere(geo, this.globeRadius);
    const targetPos = new THREE.Vector3(target.x, target.y, target.z).normalize();
    const distance = this.defaultDistance || this.globeRadius * 2.6;
    const destination = targetPos.multiplyScalar(distance);
    this.focusTween = {
      start: performance.now(),
      duration: 1200,
      from: this.camera.position.clone(),
      to: destination,
    };
    this.controls.target.set(0, 0, 0);
  }

  updateFocus() {
    if (!this.focusTween || !this.camera) return;
    const now = performance.now();
    const elapsed = now - this.focusTween.start;
    const t = Math.min(1, elapsed / this.focusTween.duration);
    const eased = t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
    this.camera.position.lerpVectors(this.focusTween.from, this.focusTween.to, eased);
    if (t >= 1) {
      this.focusTween = null;
    }
  }
}

class PinLayer {
  constructor(layerEl, renderer, boundsEl, popup) {
    this.layerEl = layerEl;
    this.renderer = renderer;
    this.boundsEl = boundsEl;
    this.popup = popup;
    this.nodes = new Map();
    this.bind();
  }

  bind() {
    if (!this.layerEl) return;
    this.layerEl.addEventListener("click", (event) => {
      const pin = event.target.closest(".pin");
      if (!pin) return;
      event.stopPropagation();
      const label = pin.dataset.label || pin.textContent || "Entity";
      const entityId = pin.dataset.entity;
      this.popup?.openFor(pin, entityId, label);
    });
  }

  syncPins(entities, world) {
    if (!this.layerEl) return;
    if (this.layerEl.style.display === "none") return;
    const bounds = this.boundsEl?.getBoundingClientRect?.();
    const clamp = bounds || {
      left: 0,
      top: 0,
      right: window.innerWidth,
      bottom: window.innerHeight,
    };
    const pad = 18;
    entities.forEach((entity) => {
      const pin = world.getComponent(entity, "Pin");
      if (!pin) return;
      const meta = world.getComponent(entity, "Meta");
      if (meta?.kind === "flight" || meta?.kind === "satellite" || meta?.kind === "ship") return;
      let node = this.nodes.get(entity);
      if (!node) {
        node = document.createElement("div");
        node.className = "pin";
        node.textContent = pin.label;
        node.dataset.entity = entity;
        this.layerEl.appendChild(node);
        this.nodes.set(entity, node);
      } else {
        node.textContent = pin.label;
      }
      node.dataset.label = pin.label;
      const geo = world.getComponent(entity, "Geo");
      if (!geo || !this.renderer) return;
      const pos = this.renderer.positionForGeo(geo, this.renderer.markerAltitude + 1.5);
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;
      const clampedX = Math.min(Math.max(screen.x, clamp.left + pad), clamp.right - pad);
      const clampedY = Math.min(Math.max(screen.y, clamp.top + pad), clamp.bottom - pad);
      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      if (screen.visible && withinBounds) {
        node.classList.remove("occluded");
        node.style.opacity = "1";
        node.style.pointerEvents = "auto";
        node.style.transform = `translate(${screen.x}px, ${screen.y}px) translate(-50%, -50%)`;
      } else {
        node.classList.remove("occluded");
        node.style.opacity = "0";
        node.style.pointerEvents = "none";
        node.style.transform = `translate(${clampedX}px, ${clampedY}px) translate(-50%, -50%)`;
        if (this.popup?.active === node) this.popup.closeMenu();
      }
    });
  }

  prune(world) {
    for (const [entity, node] of this.nodes.entries()) {
      if (!world.entities.has(entity)) {
        node.remove();
        this.nodes.delete(entity);
      }
    }
  }
}

class FlightPinLayer {
  constructor(layerEl, renderer, boundsEl, popup) {
    this.layerEl = layerEl;
    this.renderer = renderer;
    this.boundsEl = boundsEl;
    this.popup = popup;
    this.nodes = new Map();
    this.bind();
  }

  bind() {
    if (!this.layerEl) return;
    this.layerEl.addEventListener("click", (event) => {
      const pin = event.target.closest('.pin[data-kind="flight"]');
      if (!pin) return;
      event.stopPropagation();
      const label = pin.dataset.label || "Flight";
      const entityId = pin.dataset.entity;
      this.popup?.openFor(pin, entityId, label);
    });
  }

  setVisible(visible) {
    if (!this.layerEl) return;
    this.layerEl.style.display = visible ? "block" : "none";
  }

  syncPins(entities, world) {
    if (!this.layerEl || !this.renderer) return;
    if (this.layerEl.style.display === "none") return;
    const bounds = this.boundsEl?.getBoundingClientRect?.();
    const clamp = bounds || {
      left: 0,
      top: 0,
      right: window.innerWidth,
      bottom: window.innerHeight,
    };
    const pad = 22;
    entities.forEach((entity) => {
      const flight = world.getComponent(entity, "Flight");
      if (!flight) return;
      let node = this.nodes.get(entity);
      if (!node) {
        node = document.createElement("div");
        node.className = "pin";
        node.dataset.kind = "flight";
        node.dataset.entity = entity;
        node.addEventListener("click", (event) => {
          event.stopPropagation();
          const label = node.dataset.label || "Flight";
          const entityId = node.dataset.entity;
          this.popup?.openFor(node, entityId, label);
        });
        this.layerEl.appendChild(node);
        this.nodes.set(entity, node);
      }
      const label = formatFlightLabel(flight);
      node.dataset.label = label;
      const details = formatFlightDetails(flight);
      if (details) {
        node.dataset.details = details;
        node.title = `${label} · ${details}`;
      } else {
        node.dataset.details = "";
        node.title = label;
      }
      node.dataset.status = flight.on_ground ? "ground" : "airborne";
      node.textContent = label;
      const heading = Number.isFinite(flight.heading_deg) ? flight.heading_deg : 0;
      node.style.setProperty("--heading", `${heading}deg`);
      const geo = world.getComponent(entity, "Geo");
      if (!geo) return;
      const altitudeKm = Number.isFinite(flight.altitude_m)
        ? flight.altitude_m / 1000
        : 8;
      const altitude = Math.min(
        8,
        Math.max(0.6, altitudeKm * FLIGHT_CONFIG.altitudeScale),
      );
      const pos = this.renderer.positionForGeo(
        geo,
        this.renderer.markerAltitude + altitude,
      );
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;
      const clampedX = Math.min(Math.max(screen.x, clamp.left + pad), clamp.right - pad);
      const clampedY = Math.min(Math.max(screen.y, clamp.top + pad), clamp.bottom - pad);
      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      if (screen.visible && withinBounds) {
        node.style.opacity = "1";
        node.style.pointerEvents = "auto";
        node.style.transform = `translate(${screen.x}px, ${screen.y}px) translate(-50%, -50%)`;
      } else {
        node.style.opacity = "0";
        node.style.pointerEvents = "none";
        node.style.transform = `translate(${clampedX}px, ${clampedY}px) translate(-50%, -50%)`;
        if (this.popup?.active === node) this.popup.closeMenu();
      }
    });
  }

  prune(world) {
    for (const [entity, node] of this.nodes.entries()) {
      if (!world.entities.has(entity)) {
        node.remove();
        this.nodes.delete(entity);
      }
    }
  }
}

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
    const altitudeKm = Number.isFinite(flight.altitude_m)
      ? flight.altitude_m / 1000
      : 8;
    return Math.min(
      8,
      Math.max(0.6, altitudeKm * FLIGHT_CONFIG.altitudeScale),
    );
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
    const altitudeKm = Number.isFinite(flight.altitude_m)
      ? flight.altitude_m / 1000
      : 8;
    return Math.min(
      8,
      Math.max(0.6, altitudeKm * FLIGHT_CONFIG.altitudeScale),
    );
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

  sync(entities, world) {
    if (!this.renderer) return;
    const seen = new Set();
    const scale = this.scaleForDistance();
    entities.forEach((entity) => {
      const flight = world.getComponent(entity, "Flight");
      if (!flight) return;
      const mesh = this.ensureMesh(entity);
      const altitude = this.altitudeForFlight(flight);
      const geo = world.getComponent(entity, "Geo");
      if (!geo) return;
      const pos = this.renderer.positionForGeo(
        geo,
        this.renderer.markerAltitude + altitude,
      );
      mesh.position.set(pos.x, pos.y, pos.z);
      mesh.scale.set(scale, scale, scale);
      const heading = Number.isFinite(flight.heading_deg) ? flight.heading_deg : 0;
      const { basis } = this.buildOrientation(geo.lat, geo.lon, heading);
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
  constructor(renderer, world, layerEl, edgeLayer) {
    this.renderer = renderer;
    this.world = world;
    this.visible = false;
    this.pinLayer = new FlightPinLayer(layerEl, renderer, els.board, edgeLayer);
    this.trails = new FlightTrailLayer(renderer);
    this.planes = new FlightMeshLayer(renderer);
    this.lastSnapshot = null;
    this.pinLayer.setVisible(false);
    this.trails.setVisible(false);
    this.planes.setVisible(false);
  }

  setVisible(visible) {
    this.visible = visible;
    this.pinLayer.setVisible(visible);
    this.trails.setVisible(visible && this.renderer?.mode === "globe");
    this.planes.setVisible(visible && this.renderer?.mode === "globe");
  }

  ingest(snapshot) {
    if (!snapshot) return;
    this.lastSnapshot = snapshot;
    syncFlights(snapshot, this.world);
    this.trails.ingest(snapshot.flights || []);
  }

  sync() {
    if (!this.visible) return;
    const flights = this.world.query(["Geo", "Flight"]);
    this.pinLayer.syncPins(flights, this.world);
    this.pinLayer.prune(this.world);
    this.trails.setVisible(this.visible && this.renderer?.mode === "globe");
    this.planes.setVisible(this.visible && this.renderer?.mode === "globe");
    this.planes.sync(flights, this.world);
  }
}

class SatellitePinLayer {
  constructor(layerEl, renderer, boundsEl, popup) {
    this.layerEl = layerEl;
    this.renderer = renderer;
    this.boundsEl = boundsEl;
    this.popup = popup;
    this.nodes = new Map();
    this.bind();
  }

  bind() {
    if (!this.layerEl) return;
    this.layerEl.addEventListener("click", (event) => {
      const pin = event.target.closest('.pin[data-kind="satellite"]');
      if (!pin) return;
      event.stopPropagation();
      const label = pin.dataset.label || "Satellite";
      const entityId = pin.dataset.entity;
      this.popup?.openFor(pin, entityId, label);
    });
  }

  setVisible(visible) {
    if (!this.layerEl) return;
    this.layerEl.style.display = visible ? "block" : "none";
  }

  syncPins(entities, world) {
    if (!this.layerEl || !this.renderer) return;
    if (this.layerEl.style.display === "none") return;
    const bounds = this.boundsEl?.getBoundingClientRect?.();
    const clamp = bounds || {
      left: 0,
      top: 0,
      right: window.innerWidth,
      bottom: window.innerHeight,
    };
    const pad = 22;
    entities.forEach((entity) => {
      const satellite = world.getComponent(entity, "Satellite");
      if (!satellite) return;
      let node = this.nodes.get(entity);
      if (!node) {
        node = document.createElement("div");
        node.className = "pin";
        node.dataset.kind = "satellite";
        node.dataset.entity = entity;
        node.addEventListener("click", (event) => {
          event.stopPropagation();
          const label = node.dataset.label || "Satellite";
          const entityId = node.dataset.entity;
          this.popup?.openFor(node, entityId, label);
        });
        this.layerEl.appendChild(node);
        this.nodes.set(entity, node);
      }
      const label = formatSatelliteLabel(satellite);
      node.dataset.label = label;
      const details = formatSatelliteDetails(satellite);
      if (details) {
        node.dataset.details = details;
        node.title = `${label} · ${details}`;
      } else {
        node.dataset.details = "";
        node.title = label;
      }
      node.dataset.orbit = orbitBandForSatellite(satellite);
      node.textContent = label;
      const geo = world.getComponent(entity, "Geo");
      if (!geo) return;
      const altitude = altitudeForSatellite(satellite);
      const pos = this.renderer.positionForGeo(
        geo,
        this.renderer.markerAltitude + altitude,
      );
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;
      const clampedX = Math.min(Math.max(screen.x, clamp.left + pad), clamp.right - pad);
      const clampedY = Math.min(Math.max(screen.y, clamp.top + pad), clamp.bottom - pad);
      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      if (screen.visible && withinBounds) {
        node.style.opacity = "1";
        node.style.pointerEvents = "auto";
        node.style.transform = `translate(${screen.x}px, ${screen.y}px) translate(-50%, -50%)`;
      } else {
        node.style.opacity = "0";
        node.style.pointerEvents = "none";
        node.style.transform = `translate(${clampedX}px, ${clampedY}px) translate(-50%, -50%)`;
        if (this.popup?.active === node) this.popup.closeMenu();
      }
    });
  }

  prune(world) {
    for (const [entity, node] of this.nodes.entries()) {
      if (!world.entities.has(entity)) {
        node.remove();
        this.nodes.delete(entity);
      }
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

  sync(entities, world) {
    if (!this.renderer) return;
    const seen = new Set();
    const scale = this.scaleForDistance();
    entities.forEach((entity) => {
      const satellite = world.getComponent(entity, "Satellite");
      if (!satellite) return;
      const mesh = this.ensureMesh(entity);
      const geo = world.getComponent(entity, "Geo");
      if (!geo) return;
      const altitude = altitudeForSatellite(satellite);
      const pos = this.renderer.positionForGeo(
        geo,
        this.renderer.markerAltitude + altitude,
      );
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
  constructor(renderer, world, layerEl, edgeLayer) {
    this.renderer = renderer;
    this.world = world;
    this.visible = false;
    this.pinLayer = new SatellitePinLayer(layerEl, renderer, els.board, edgeLayer);
    this.meshes = new SatelliteMeshLayer(renderer);
    this.lastSnapshot = null;
    this.pinLayer.setVisible(false);
    this.meshes.setVisible(false);
  }

  setVisible(visible) {
    this.visible = visible;
    this.pinLayer.setVisible(visible);
    this.meshes.setVisible(visible && this.renderer?.mode === "globe");
  }

  ingest(snapshot) {
    if (!snapshot) return;
    this.lastSnapshot = snapshot;
    syncSatellites(snapshot, this.world);
  }

  sync() {
    if (!this.visible) return;
    const satellites = this.world.query(["Geo", "Satellite"]);
    this.pinLayer.syncPins(satellites, this.world);
    this.pinLayer.prune(this.world);
    this.meshes.setVisible(this.visible && this.renderer?.mode === "globe");
    this.meshes.sync(satellites, this.world);
  }
}

class ShipPinLayer {
  constructor(layerEl, renderer, boundsEl, popup) {
    this.layerEl = layerEl;
    this.renderer = renderer;
    this.boundsEl = boundsEl;
    this.popup = popup;
    this.nodes = new Map();
    this.bind();
  }

  bind() {
    if (!this.layerEl) return;
    this.layerEl.addEventListener("click", (event) => {
      const pin = event.target.closest('.pin[data-kind="ship"]');
      if (!pin) return;
      event.stopPropagation();
      const label = pin.dataset.label || "Ship";
      const entityId = pin.dataset.entity;
      this.popup?.openFor(pin, entityId, label);
    });
  }

  setVisible(visible) {
    if (!this.layerEl) return;
    this.layerEl.style.display = visible ? "block" : "none";
  }

  syncPins(entities, world) {
    if (!this.layerEl || !this.renderer) return;
    if (this.layerEl.style.display === "none") return;
    const bounds = this.boundsEl?.getBoundingClientRect?.();
    const clamp = bounds || {
      left: 0,
      top: 0,
      right: window.innerWidth,
      bottom: window.innerHeight,
    };
    const pad = 22;
    const baseAltitude = shipBaseAltitude(this.renderer);
    entities.forEach((entity) => {
      const ship = world.getComponent(entity, "Ship");
      if (!ship) return;
      let node = this.nodes.get(entity);
      if (!node) {
        node = document.createElement("div");
        node.className = "pin";
        node.dataset.kind = "ship";
        node.dataset.entity = entity;
        node.addEventListener("click", (event) => {
          event.stopPropagation();
          const label = node.dataset.label || "Ship";
          const entityId = node.dataset.entity;
          this.popup?.openFor(node, entityId, label);
        });
        this.layerEl.appendChild(node);
        this.nodes.set(entity, node);
      }
      const label = formatShipLabel(ship);
      node.dataset.label = label;
      const details = formatShipDetails(ship);
      if (details) {
        node.dataset.details = details;
        node.title = `${label} · ${details}`;
      } else {
        node.dataset.details = "";
        node.title = label;
      }
      node.dataset.vessel = vesselGroupForShip(ship);
      node.textContent = label;
      const geo = world.getComponent(entity, "Geo");
      if (!geo) return;
      const pos = this.renderer.positionForGeo(
        geo,
        baseAltitude + altitudeForShip(ship),
      );
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;
      const clampedX = Math.min(Math.max(screen.x, clamp.left + pad), clamp.right - pad);
      const clampedY = Math.min(Math.max(screen.y, clamp.top + pad), clamp.bottom - pad);
      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      if (screen.visible && withinBounds) {
        node.style.opacity = "1";
        node.style.pointerEvents = "auto";
        node.style.transform = `translate(${screen.x}px, ${screen.y}px) translate(-50%, -50%)`;
      } else {
        node.style.opacity = "0";
        node.style.pointerEvents = "none";
        node.style.transform = `translate(${clampedX}px, ${clampedY}px) translate(-50%, -50%)`;
        if (this.popup?.active === node) this.popup.closeMenu();
      }
    });
  }

  prune(world) {
    for (const [entity, node] of this.nodes.entries()) {
      if (!world.entities.has(entity)) {
        node.remove();
        this.nodes.delete(entity);
      }
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

  sync(entities, world) {
    if (!this.renderer) return;
    const seen = new Set();
    const scale = this.scaleForDistance();
    const baseAltitude = shipBaseAltitude(this.renderer);
    entities.forEach((entity) => {
      const ship = world.getComponent(entity, "Ship");
      if (!ship) return;
      const mesh = this.ensureMesh(entity);
      const geo = world.getComponent(entity, "Geo");
      if (!geo) return;
      const pos = this.renderer.positionForGeo(
        geo,
        baseAltitude + altitudeForShip(ship),
      );
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
  constructor(renderer, world, layerEl, edgeLayer) {
    this.renderer = renderer;
    this.world = world;
    this.visible = false;
    this.pinLayer = new ShipPinLayer(layerEl, renderer, els.board, edgeLayer);
    this.meshes = new ShipMeshLayer(renderer);
    this.lastSnapshot = null;
    this.pinLayer.setVisible(false);
    this.meshes.setVisible(false);
  }

  setVisible(visible) {
    this.visible = visible;
    this.pinLayer.setVisible(visible);
    this.meshes.setVisible(visible && this.renderer?.mode === "globe");
  }

  ingest(snapshot) {
    if (!snapshot) return;
    this.lastSnapshot = snapshot;
    syncShips(snapshot, this.world);
  }

  sync() {
    if (!this.visible) return;
    const ships = this.world.query(["Geo", "Ship"]);
    this.pinLayer.syncPins(ships, this.world);
    this.pinLayer.prune(this.world);
    this.meshes.setVisible(this.visible && this.renderer?.mode === "globe");
    this.meshes.sync(ships, this.world);
  }
}

const hashString = (value) => {
  let hash = 2166136261;
  for (let i = 0; i < value.length; i += 1) {
    hash ^= value.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
};

const hashToGeo = (id) => {
  const h1 = hashString(id);
  const h2 = hashString(`${id}:lon`);
  const lat = (h1 / 0xffffffff) * 180 - 90;
  const lon = (h2 / 0xffffffff) * 360 - 180;
  return { lat, lon };
};

const geoToPlane = (geo, plane) => ({
  x: (geo.lon / 180) * (plane.width / 2),
  y: 0,
  z: (-geo.lat / 90) * (plane.height / 2),
});

const geoToSphere = (geo, radius) => {
  const phi = (90 - geo.lat) * (Math.PI / 180);
  const theta = (geo.lon + 180) * (Math.PI / 180);
  const x = radius * Math.sin(phi) * Math.cos(theta);
  const y = radius * Math.cos(phi);
  const z = radius * Math.sin(phi) * Math.sin(theta);
  return { x, y, z };
};

const colorForAsset = (asset) => {
  switch (asset.status) {
    case "lost":
      return "#dc2626";
    case "degraded":
      return "#f59e0b";
    case "maintenance":
      return "#0ea5e9";
    case "assigned":
    case "available":
    default:
      return "#16a34a";
  }
};

const colorForUnit = (unit) => {
  switch (unit.readiness) {
    case "unavailable":
      return "#dc2626";
    case "degraded":
      return "#f97316";
    case "limited":
      return "#f59e0b";
    case "ready":
    default:
      return "#22c55e";
  }
};

const colorForMission = (mission) => {
  switch (mission.status) {
    case "active":
      return "#38bdf8";
    case "suspended":
      return "#f97316";
    case "completed":
      return "#94a3b8";
    case "aborted":
      return "#dc2626";
    case "planned":
    default:
      return "#64748b";
  }
};

const colorForIncident = (incident) => {
  switch (incident.status) {
    case "responding":
      return "#ef4444";
    case "reported":
    case "verified":
      return "#f97316";
    case "contained":
    case "resolved":
    case "closed":
      return "#94a3b8";
    default:
      return "#f59e0b";
  }
};

const colorForFlight = (flight) => {
  if (flight?.on_ground) return "#94a3b8";
  return "#38bdf8";
};

const formatFlightLabel = (flight) => {
  if (!flight) return "FL";
  const callsign = flight.callsign?.trim?.();
  if (callsign) return callsign;
  const id = flight.id?.split?.(":").pop?.();
  return id || "FL";
};

const formatFlightDetails = (flight) => {
  if (!flight) return "";
  const parts = [];
  if (Number.isFinite(flight.altitude_m)) {
    const meters = Math.round(flight.altitude_m);
    const feet = Math.round(meters * 3.28084);
    parts.push(`${meters} m (${feet} ft)`);
  }
  if (Number.isFinite(flight.velocity_mps)) {
    const knots = Math.round(flight.velocity_mps * 1.94384);
    parts.push(`${knots} kt`);
  }
  if (Number.isFinite(flight.heading_deg)) {
    parts.push(`${Math.round(flight.heading_deg)}°`);
  }
  return parts.join(" · ");
};

const orbitBandForSatellite = (satellite) => {
  const altitude = satellite?.altitude_km;
  if (!Number.isFinite(altitude)) return "unknown";
  if (altitude < 2000) return "leo";
  if (altitude < 35786) return "meo";
  return "geo";
};

const colorForSatellite = (satellite) => {
  switch (orbitBandForSatellite(satellite)) {
    case "leo":
      return "#facc15";
    case "meo":
      return "#38bdf8";
    case "geo":
      return "#a3e635";
    default:
      return "#f59e0b";
  }
};

const altitudeForSatellite = (satellite) => {
  const altitudeKm = Number.isFinite(satellite?.altitude_km)
    ? satellite.altitude_km
    : 550;
  const scaled = altitudeKm * SATELLITE_CONFIG.altitudeScale;
  return Math.min(
    SATELLITE_CONFIG.altitudeMax,
    Math.max(SATELLITE_CONFIG.altitudeMin, scaled),
  );
};

const formatSatelliteLabel = (satellite) => {
  if (!satellite) return "SAT";
  const name = satellite.name?.trim?.();
  if (name) return name;
  if (Number.isFinite(satellite.norad_id)) return `SAT-${satellite.norad_id}`;
  const id = satellite.id?.split?.(":").pop?.();
  return id || "SAT";
};

const formatSatelliteDetails = (satellite) => {
  if (!satellite) return "";
  const parts = [];
  if (Number.isFinite(satellite.altitude_km)) {
    parts.push(`${Math.round(satellite.altitude_km)} km`);
  }
  if (Number.isFinite(satellite.velocity_kms)) {
    parts.push(`${satellite.velocity_kms.toFixed(2)} km/s`);
  }
  if (Number.isFinite(satellite.inclination_deg)) {
    parts.push(`${Math.round(satellite.inclination_deg)}° inc`);
  }
  if (Number.isFinite(satellite.period_min)) {
    parts.push(`${Math.round(satellite.period_min)} min`);
  }
  return parts.join(" · ");
};

const vesselGroupForShip = (ship) => {
  const value = ship?.vessel_type;
  if (!Number.isFinite(value)) return "unknown";
  if (value >= 80 && value < 90) return "tanker";
  if (value >= 70 && value < 80) return "cargo";
  if (value >= 60 && value < 70) return "passenger";
  if (value >= 30 && value < 40) return "fishing";
  return "other";
};

const colorForShip = (ship) => {
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

const altitudeForShip = () =>
  Number.isFinite(SHIP_CONFIG.altitude) ? SHIP_CONFIG.altitude : 0.12;

const shipBaseAltitude = (renderer) => {
  const base = renderer?.markerAltitude ?? 3.0;
  return Math.max(0.25, base * 0.2);
};

const formatShipLabel = (ship) => {
  if (!ship) return "SHIP";
  const name = ship.name?.trim?.();
  if (name) return name;
  const callsign = ship.callsign?.trim?.();
  if (callsign) return callsign;
  if (Number.isFinite(ship.mmsi)) return `MMSI ${ship.mmsi}`;
  const id = ship.id?.split?.(":").pop?.();
  return id || "SHIP";
};

const formatShipDetails = (ship) => {
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

const syncEntities = (payload, world) => {
  if (!payload) return;
  const seen = new Set();
  const index = world.entityIndex || new Map();
  world.entityIndex = index;

  const upsert = (key, data, color, pinLabel) => {
    let entity = index.get(key);
    if (!entity) {
      entity = world.createEntity();
      index.set(key, entity);
    }
    seen.add(entity);
    const geo = hashToGeo(data.id);
    world.addComponent(entity, "Geo", geo);
    world.addComponent(entity, "Renderable", { color });
    world.addComponent(entity, "Meta", { kind: key.split(":")[0], data });
    if (pinLabel) {
      world.addComponent(entity, "Pin", { label: pinLabel });
    } else {
      world.removeComponent(entity, "Pin");
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
      world.removeEntity(entity);
    }
  }
};

const syncFlights = (payload, world) => {
  if (!payload || !Array.isArray(payload.flights)) return;
  const seen = new Set();
  const index = world.flightIndex || new Map();
  world.flightIndex = index;

  payload.flights.forEach((flight) => {
    if (!Number.isFinite(flight.lat) || !Number.isFinite(flight.lon)) return;
    const key = flight.id || `${flight.callsign || "flight"}:${flight.lat}:${flight.lon}`;
    let entity = index.get(key);
    if (!entity) {
      entity = world.createEntity();
      index.set(key, entity);
    }
    seen.add(entity);
    world.addComponent(entity, "Geo", { lat: flight.lat, lon: flight.lon });
    world.addComponent(entity, "Flight", flight);
    world.addComponent(entity, "Renderable", { color: colorForFlight(flight) });
    world.addComponent(entity, "Meta", { kind: "flight", data: flight });
    world.addComponent(entity, "Pin", {
      label: formatFlightLabel(flight),
      icon: "/static/assets/plane.png",
      heading: flight.heading_deg,
      status: flight.on_ground ? "ground" : "airborne",
    });
  });

  for (const [key, entity] of index.entries()) {
    if (!seen.has(entity)) {
      index.delete(key);
      world.removeEntity(entity);
    }
  }
};

const syncSatellites = (payload, world) => {
  if (!payload || !Array.isArray(payload.satellites)) return;
  const seen = new Set();
  const index = world.satelliteIndex || new Map();
  world.satelliteIndex = index;

  payload.satellites.forEach((satellite) => {
    if (!Number.isFinite(satellite.lat) || !Number.isFinite(satellite.lon)) return;
    const key =
      satellite.id ||
      `${satellite.norad_id || "sat"}:${satellite.lat}:${satellite.lon}`;
    let entity = index.get(key);
    if (!entity) {
      entity = world.createEntity();
      index.set(key, entity);
    }
    seen.add(entity);
    world.addComponent(entity, "Geo", { lat: satellite.lat, lon: satellite.lon });
    world.addComponent(entity, "Satellite", satellite);
    world.addComponent(entity, "Renderable", { color: colorForSatellite(satellite) });
    world.addComponent(entity, "Meta", { kind: "satellite", data: satellite });
    world.addComponent(entity, "Pin", {
      label: formatSatelliteLabel(satellite),
    });
  });

  for (const [key, entity] of index.entries()) {
    if (!seen.has(entity)) {
      index.delete(key);
      world.removeEntity(entity);
    }
  }
};

const syncShips = (payload, world) => {
  if (!payload || !Array.isArray(payload.ships)) return;
  const seen = new Set();
  const index = world.shipIndex || new Map();
  world.shipIndex = index;

  payload.ships.forEach((ship) => {
    if (!Number.isFinite(ship.lat) || !Number.isFinite(ship.lon)) return;
    const key = ship.id || `${ship.mmsi || "ship"}:${ship.lat}:${ship.lon}`;
    let entity = index.get(key);
    if (!entity) {
      entity = world.createEntity();
      index.set(key, entity);
    }
    seen.add(entity);
    world.addComponent(entity, "Geo", { lat: ship.lat, lon: ship.lon });
    world.addComponent(entity, "Ship", ship);
    world.addComponent(entity, "Renderable", { color: colorForShip(ship) });
    world.addComponent(entity, "Meta", { kind: "ship", data: ship });
    world.addComponent(entity, "Pin", { label: formatShipLabel(ship) });
  });

  for (const [key, entity] of index.entries()) {
    if (!seen.has(entity)) {
      index.delete(key);
      world.removeEntity(entity);
    }
  }
};

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

class EdgeLayer {
  constructor(layerEl, renderer, boundsEl, onAction) {
    this.layerEl = layerEl;
    this.renderer = renderer;
    this.boundsEl = boundsEl;
    this.nodes = new Map();
    this.active = null;
    this.onAction = onAction;
    this.popupBackdrop = null;
    this.popup = null;
    this.initPopup();
  }

  initPopup() {
    const backdrop = document.createElement("div");
    backdrop.className = "edge-popup-backdrop";
    const popup = document.createElement("div");
    popup.className = "edge-popup";
    backdrop.appendChild(popup);
    document.body.appendChild(backdrop);
    backdrop.addEventListener("click", () => this.closeMenu());
    popup.addEventListener("click", (event) => {
      event.stopPropagation();
      const button = event.target.closest("button[data-action]");
      if (!button || !this.active) return;
      const action = button.dataset.action;
      const entityId = this.active.dataset.entity;
      if (action && entityId) this.onAction?.(action, entityId);
      this.closeMenu();
    });
    this.popupBackdrop = backdrop;
    this.popup = popup;
  }

  bind() {
    if (!this.layerEl) return;
    this.layerEl.addEventListener("click", (event) => {
      const marker = event.target.closest(".edge-marker");
      if (!marker) return;
      event.stopPropagation();
      if (this.active === marker) {
        this.closeMenu();
        return;
      }
      this.openFor(
        marker,
        marker.dataset.entity,
        marker.dataset.label || marker.title || "Entity",
      );
    });
  }

  closeMenu() {
    if (this.active) this.active.classList.remove("selected");
    this.active = null;
    if (this.popupBackdrop) this.popupBackdrop.classList.remove("active");
  }

  openFor(node, entityId, label) {
    if (!this.popup || !this.popupBackdrop) return;
    if (this.active === node) {
      this.closeMenu();
      return;
    }
    this.closeMenu();
    this.active = node;
    this.active.classList.add("selected");
    if (entityId) this.active.dataset.entity = entityId;
    const safeLabel = label || "Entity";
    const details = node.dataset.details || "";
    const detailsHtml = details ? `<div class="edge-popup-meta">${details}</div>` : "";
    this.popup.innerHTML = `
      <div class="edge-popup-title">${safeLabel}</div>
      ${detailsHtml}
      <div class="edge-popup-actions">
        <button data-action="focus">Focus</button>
        <button data-action="details">Details</button>
        <button data-action="task">Task</button>
      </div>
    `;
    this.popupBackdrop.classList.add("active");
  }

  createNode(entityId) {
    const node = document.createElement("div");
    node.className = "edge-marker";
    node.dataset.entity = entityId;
    node.innerHTML = `<span class="edge-symbol"></span>`;
    return node;
  }

  syncEdges(entities, world) {
    if (!this.layerEl || !this.renderer) return;
    const bounds = this.boundsEl?.getBoundingClientRect?.();
    const clamp = bounds || {
      left: 0,
      top: 0,
      right: window.innerWidth,
      bottom: window.innerHeight,
      width: window.innerWidth,
      height: window.innerHeight,
    };
    const pad = 22;
    const centerX = clamp.left + clamp.width / 2;
    const centerY = clamp.top + clamp.height / 2;
    const edgeX = clamp.width / 2 - pad;
    const edgeY = clamp.height / 2 - pad;

    entities.forEach((entity) => {
      const geo = world.getComponent(entity, "Geo");
      const meta = world.getComponent(entity, "Meta");
      const pin = world.getComponent(entity, "Pin");
      if (!geo || !meta) return;
      if (!pin) return;
      const pos = this.renderer.positionForGeo(geo, this.renderer.markerAltitude + 2.5);
      const screen = this.renderer.projectToScreen(pos);
      if (!screen) return;

      const withinBounds =
        screen.x >= clamp.left + pad &&
        screen.x <= clamp.right - pad &&
        screen.y >= clamp.top + pad &&
        screen.y <= clamp.bottom - pad;
      if (screen.visible && withinBounds) {
        const existing = this.nodes.get(entity);
        if (existing) {
          existing.style.opacity = "0";
          existing.style.pointerEvents = "none";
          if (this.active === existing) this.closeMenu();
        }
        return;
      }

      let node = this.nodes.get(entity);
      if (!node) {
        node = this.createNode(entity);
        this.nodes.set(entity, node);
        this.layerEl.appendChild(node);
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
      node.style.opacity = "1";
      node.style.pointerEvents = "auto";
      node.style.transform = `translate(${x}px, ${y}px) translate(-50%, -50%)`;
      node.classList.toggle("occluded", screen.behind);
      node.classList.toggle("edge-marker--flight", meta.kind === "flight");
      node.classList.toggle("edge-marker--satellite", meta.kind === "satellite");
      node.classList.toggle("edge-marker--ship", meta.kind === "ship");
      const symbol = collapseLabel(pin.label) || edgeSymbolFor(meta);
      node.querySelector(".edge-symbol").textContent = symbol;
      const details =
        meta.kind === "flight"
          ? formatFlightDetails(meta.data)
          : meta.kind === "satellite"
            ? formatSatelliteDetails(meta.data)
            : meta.kind === "ship"
              ? formatShipDetails(meta.data)
            : "";
      node.dataset.details = details || "";
      node.title = pin.label || meta.data?.name || meta.data?.summary || meta.kind;
      node.dataset.label = node.title;
    });
  }

  prune(world) {
    for (const [entity, node] of this.nodes.entries()) {
      if (!world.entities.has(entity)) {
        node.remove();
        this.nodes.delete(entity);
      }
    }
  }
}

const setupDockToggles = () => {
  document.querySelectorAll("[data-dock-toggle]").forEach((button) => {
    button.addEventListener("click", () => {
      const target = button.dataset.dockToggle;
      const dock = target === "left" ? els.dockLeft : els.dockRight;
      if (!dock) return;
      toggleDockState(dock);
    });
  });
};

const dockStates = ["open", "minimized", "closed"];
let dockZ = 20;

const normalizeDockState = (state) =>
  dockStates.includes(state) ? state : "open";

const bringDockToFront = (dock) => {
  dockZ += 1;
  dock.style.zIndex = dockZ.toString();
};

const setDockState = (dock, state) => {
  if (!dock) return;
  const next = normalizeDockState(state);
  dock.dataset.state = next;
  dock.setAttribute("aria-hidden", next === "open" ? "false" : "true");
  if (next === "open") bringDockToFront(dock);
  updateWindowMenuState();
};

const toggleDockState = (dock) => {
  if (!dock) return;
  const state = normalizeDockState(dock.dataset.state);
  const next = state === "open" ? "minimized" : "open";
  setDockState(dock, next);
};

const updateWindowMenuState = () => {
  document.querySelectorAll("[data-window-state]").forEach((node) => {
    const id = node.dataset.windowState;
    const dock = document.getElementById(id);
    if (!dock) return;
    const state = normalizeDockState(dock.dataset.state);
    node.dataset.state = state;
    node.textContent =
      state === "open" ? "Open" : state === "minimized" ? "Minimized" : "Closed";
  });
};

const applyDockAction = (dock, action) => {
  if (!dock) return;
  if (action === "minimize") {
    setDockState(dock, "minimized");
    return;
  }
  if (action === "close") {
    setDockState(dock, "closed");
    return;
  }
  if (action === "open") {
    setDockState(dock, "open");
  }
};

const allDocks = () => [els.dockLeft, els.dockRight].filter(Boolean);

const setupDockControls = () => {
  document.querySelectorAll("[data-dock-action]").forEach((button) => {
    const action = button.dataset.dockAction;
    const dock = button.closest(".dock");
    if (!action || !dock) return;
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      applyDockAction(dock, action);
    });
  });
};

const setupDockDrag = () => {
  document.querySelectorAll(".dock").forEach((dock) => {
    dock.addEventListener("pointerdown", () => bringDockToFront(dock));
  });
  document.querySelectorAll(".dock-header").forEach((header) => {
    header.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      if (event.target.closest(".dock-controls")) return;
      const dock = header.closest(".dock");
      if (!dock || normalizeDockState(dock.dataset.state) !== "open") return;
      event.preventDefault();
      bringDockToFront(dock);
      const rect = dock.getBoundingClientRect();
      const offsetX = event.clientX - rect.left;
      const offsetY = event.clientY - rect.top;
      dock.style.left = `${rect.left}px`;
      dock.style.top = `${rect.top}px`;
      dock.style.right = "auto";
      dock.style.bottom = "auto";
      dock.classList.add("dragging");

      const onMove = (moveEvent) => {
        const width = dock.offsetWidth;
        const height = dock.offsetHeight;
        const maxLeft = Math.max(12, window.innerWidth - width - 12);
        const maxTop = Math.max(12, window.innerHeight - height - 12);
        const nextLeft = Math.min(
          maxLeft,
          Math.max(12, moveEvent.clientX - offsetX),
        );
        const nextTop = Math.min(
          maxTop,
          Math.max(12, moveEvent.clientY - offsetY),
        );
        dock.style.left = `${nextLeft}px`;
        dock.style.top = `${nextTop}px`;
      };

      const onUp = () => {
        dock.classList.remove("dragging");
        window.removeEventListener("pointermove", onMove);
      };

      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp, { once: true });
    });
  });
};

const closePillMenus = () => {
  document.querySelectorAll(".pill-menu").forEach((menu) => {
    menu.dataset.open = "false";
    const trigger = menu.querySelector(".pill-menu-trigger");
    if (trigger) trigger.setAttribute("aria-expanded", "false");
  });
};

const setupPillMenus = () => {
  const menus = document.querySelectorAll(".pill-menu");
  if (!menus.length) return;
  menus.forEach((menu) => {
    const trigger = menu.querySelector(".pill-menu-trigger");
    if (!trigger) return;
    trigger.addEventListener("click", (event) => {
      event.stopPropagation();
      const isOpen = menu.dataset.open === "true";
      closePillMenus();
      if (!isOpen) {
        menu.dataset.open = "true";
        trigger.setAttribute("aria-expanded", "true");
      }
    });
  });

  document.addEventListener("click", (event) => {
    if (!event.target.closest(".pill-menu")) closePillMenus();
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") closePillMenus();
  });
};

const setupWindowMenuActions = () => {
  document.querySelectorAll("[data-window-action]").forEach((button) => {
    const action = button.dataset.windowAction;
    const target = button.dataset.windowId;
    button.addEventListener("click", () => {
      if (action === "toggle" && target) {
        const dock = document.getElementById(target);
        const state = normalizeDockState(dock?.dataset?.state);
        setDockState(dock, state === "open" ? "minimized" : "open");
        closePillMenus();
        return;
      }
      if (action === "open-all") {
        allDocks().forEach((dock) => setDockState(dock, "open"));
        closePillMenus();
        return;
      }
      if (action === "minimize-all") {
        allDocks().forEach((dock) => setDockState(dock, "minimized"));
        closePillMenus();
        return;
      }
      if (action === "close-all") {
        allDocks().forEach((dock) => setDockState(dock, "closed"));
        closePillMenus();
        return;
      }
    });
  });
};

const setupPanelCollapsibles = () => {
  if (!els.dockLeft) return;
  els.dockLeft
    .querySelectorAll('.panel-section[data-collapsible="true"]')
    .forEach((section) => {
      const header = section.querySelector(".panel-header");
      if (!header) return;
      const icon = header.querySelector(".panel-toggle-icon");
      const setCollapsed = (collapsed) => {
        section.dataset.collapsed = collapsed ? "true" : "false";
        header.setAttribute("aria-expanded", (!collapsed).toString());
        if (icon) icon.textContent = collapsed ? "+" : "-";
      };
      setCollapsed(section.dataset.collapsed === "true");
      header.addEventListener("click", () => {
        const next = section.dataset.collapsed !== "true";
        setCollapsed(next);
      });
    });
};

const setupLayerToggles = () => {
  document.querySelectorAll("[data-layer-toggle]").forEach((button) => {
    button.addEventListener("click", () => {
      const layerName = button.dataset.layerToggle;
      if (!layerName) return;
      const layer = document.querySelector(`[data-layer="${layerName}"]`);
      if (!layer) return;
      const hidden = layer.getAttribute("data-hidden") === "true";
      layer.setAttribute("data-hidden", (!hidden).toString());
      layer.style.display = hidden ? "block" : "none";
    });
  });
};

const setupGlobeControls = (renderer3d) => {
  document.querySelectorAll("[data-globe-mode]").forEach((button) => {
    button.addEventListener("click", () => {
      const mode = button.dataset.globeMode;
      if (!mode) return;
      renderer3d.setLightingMode(mode);
      document.querySelectorAll("[data-globe-mode]").forEach((peer) => {
        peer.dataset.active = peer === button ? "true" : "false";
      });
    });
  });

  document.querySelectorAll("[data-globe-toggle]").forEach((button) => {
    button.addEventListener("click", () => {
      const key = button.dataset.globeToggle;
      if (!key) return;
      const next = button.getAttribute("aria-pressed") !== "true";
      button.setAttribute("aria-pressed", next.toString());
      if (key === "clouds") renderer3d.setCloudsVisible(next);
      if (key === "axes") renderer3d.setAxesVisible(next);
      if (key === "grid") renderer3d.setGridVisible(next);
    });
  });
};

const formatWeatherLabel = (value) => {
  if (!value) return "";
  return value
    .replace(/_/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/^./, (char) => char.toUpperCase());
};

const setupWeatherControls = (renderer3d) => {
  const panel = document.getElementById("weather-panel");
  if (!WEATHER_CONFIG.enabled) {
    if (panel) panel.style.display = "none";
    return;
  }
  if (panel) panel.style.display = "block";
  const select = document.getElementById("weather-field");
  if (select) {
    select.innerHTML = "";
    WEATHER_CONFIG.fields.forEach((field) => {
      const option = document.createElement("option");
      option.value = field;
      option.textContent = formatWeatherLabel(field);
      select.appendChild(option);
    });
    select.value = WEATHER_CONFIG.defaultField;
    renderer3d.setWeatherField(WEATHER_CONFIG.defaultField);
    select.addEventListener("change", () => {
      renderer3d.setWeatherField(select.value);
    });
  }
  const toggle = document.querySelector("[data-weather-toggle]");
  if (toggle) {
    toggle.setAttribute("aria-pressed", "false");
    toggle.addEventListener("click", () => {
      const next = toggle.getAttribute("aria-pressed") !== "true";
      toggle.setAttribute("aria-pressed", next.toString());
      renderer3d.setWeatherVisible(next);
    });
  }
};

const clampFlightLat = (lat) => Math.max(-85, Math.min(85, lat));
const clampFlightLon = (lon) => Math.max(-180, Math.min(180, lon));
const clampShipLat = (lat) => Math.max(-85, Math.min(85, lat));
const clampShipLon = (lon) => Math.max(-180, Math.min(180, lon));

const computeFlightBounds = (renderer3d) => {
  if (!renderer3d || renderer3d.mode !== "globe") return null;
  const center = renderer3d.geoFromScreen(
    renderer3d.size.width / 2,
    renderer3d.size.height / 2,
  );
  if (!center) return null;
  const distance = renderer3d.camera?.position?.length?.() || renderer3d.defaultDistance;
  const denom = Math.max(1, renderer3d.defaultDistance - renderer3d.globeRadius);
  const ratio = Math.max(
    0.15,
    Math.min(1, (distance - renderer3d.globeRadius) / denom),
  );
  const span =
    FLIGHT_CONFIG.spanMinDeg +
    (FLIGHT_CONFIG.spanMaxDeg - FLIGHT_CONFIG.spanMinDeg) * ratio;
  const half = span / 2;
  return {
    lamin: clampFlightLat(center.lat - half),
    lamax: clampFlightLat(center.lat + half),
    lomin: clampFlightLon(center.lon - half),
    lomax: clampFlightLon(center.lon + half),
  };
};

const computeShipBounds = (renderer3d) => {
  if (!renderer3d || renderer3d.mode !== "globe") return null;
  const center = renderer3d.geoFromScreen(
    renderer3d.size.width / 2,
    renderer3d.size.height / 2,
  );
  if (!center) return null;
  const distance = renderer3d.camera?.position?.length?.() || renderer3d.defaultDistance;
  const denom = Math.max(1, renderer3d.defaultDistance - renderer3d.globeRadius);
  const ratio = Math.max(
    0.15,
    Math.min(1, (distance - renderer3d.globeRadius) / denom),
  );
  const span =
    SHIP_CONFIG.spanMinDeg + (SHIP_CONFIG.spanMaxDeg - SHIP_CONFIG.spanMinDeg) * ratio;
  const half = span / 2;
  return {
    lamin: clampShipLat(center.lat - half),
    lamax: clampShipLat(center.lat + half),
    lomin: clampShipLon(center.lon - half),
    lomax: clampShipLon(center.lon + half),
  };
};

const fetchFlights = async (renderer3d, bus, overlay) => {
  if (!FLIGHT_CONFIG.enabled || !overlay?.visible) return;
  if (!renderer3d || renderer3d.mode !== "globe") return;
  const bounds = computeFlightBounds(renderer3d);
  const params = new URLSearchParams();
  if (bounds) {
    params.set("lamin", bounds.lamin.toFixed(4));
    params.set("lomin", bounds.lomin.toFixed(4));
    params.set("lamax", bounds.lamax.toFixed(4));
    params.set("lomax", bounds.lomax.toFixed(4));
  }
  params.set("limit", FLIGHT_CONFIG.maxFlights.toString());
  try {
    const response = await fetch(`/ui/flights?${params.toString()}`, {
      cache: "no-store",
    });
    if (!response.ok) return;
    const payload = await response.json();
    bus.emit("flights:update", payload);
  } catch {
    // ignore flight fetch errors
  }
};

const fetchSatellites = async (renderer3d, bus, overlay) => {
  if (!SATELLITE_CONFIG.enabled || !overlay?.visible) return;
  if (!renderer3d || renderer3d.mode !== "globe") return;
  const params = new URLSearchParams();
  params.set("limit", SATELLITE_CONFIG.maxSatellites.toString());
  try {
    const response = await fetch(`/ui/satellites?${params.toString()}`, {
      cache: "no-store",
    });
    if (!response.ok) return;
    const payload = await response.json();
    bus.emit("satellites:update", payload);
  } catch {
    // ignore satellite fetch errors
  }
};

const fetchShips = async (renderer3d, bus, overlay) => {
  if (!SHIP_CONFIG.enabled || !overlay?.visible) return;
  if (!renderer3d || renderer3d.mode !== "globe") return;
  const bounds = computeShipBounds(renderer3d);
  const params = new URLSearchParams();
  if (bounds) {
    params.set("lamin", bounds.lamin.toFixed(4));
    params.set("lomin", bounds.lomin.toFixed(4));
    params.set("lamax", bounds.lamax.toFixed(4));
    params.set("lomax", bounds.lomax.toFixed(4));
  }
  params.set("limit", SHIP_CONFIG.maxShips.toString());
  try {
    const response = await fetch(`/ui/ships?${params.toString()}`, {
      cache: "no-store",
    });
    if (!response.ok) return;
    const payload = await response.json();
    bus.emit("ships:update", payload);
  } catch {
    // ignore ship fetch errors
  }
};

const setupFlightControls = (renderer3d, bus, overlay) => {
  const panel = document.getElementById("flight-panel");
  if (!panel || !FLIGHT_CONFIG.enabled) {
    if (panel) panel.style.display = "none";
    return;
  }
  if (els.flightProviderLabel) {
    const providerName = FLIGHT_CONFIG.source || FLIGHT_CONFIG.provider;
    els.flightProviderLabel.textContent = `Live flight tracks via ${providerName}.`;
  }
  const toggle = document.querySelector("[data-flight-toggle]");
  if (toggle) {
    toggle.setAttribute("aria-pressed", "false");
    toggle.addEventListener("click", () => {
      const next = toggle.getAttribute("aria-pressed") !== "true";
      toggle.setAttribute("aria-pressed", next.toString());
      overlay?.setVisible(next);
      if (next) {
        fetchFlights(renderer3d, bus, overlay);
      }
    });
  }
};

const setupSatelliteControls = (renderer3d, bus, overlay) => {
  const panel = document.getElementById("satellite-panel");
  if (!panel || !SATELLITE_CONFIG.enabled) {
    if (panel) panel.style.display = "none";
    return;
  }
  if (els.satelliteProviderLabel) {
    const providerName = SATELLITE_CONFIG.source || SATELLITE_CONFIG.provider;
    els.satelliteProviderLabel.textContent = `Live satellite tracks via ${providerName}.`;
  }
  const toggle = document.querySelector("[data-satellite-toggle]");
  if (toggle) {
    toggle.setAttribute("aria-pressed", "false");
    toggle.addEventListener("click", () => {
      const next = toggle.getAttribute("aria-pressed") !== "true";
      toggle.setAttribute("aria-pressed", next.toString());
      overlay?.setVisible(next);
      if (next) {
        fetchSatellites(renderer3d, bus, overlay);
      }
    });
  }
};

const setupShipControls = (renderer3d, bus, overlay) => {
  const panel = document.getElementById("ship-panel");
  if (!panel || !SHIP_CONFIG.enabled) {
    if (panel) panel.style.display = "none";
    return;
  }
  if (els.shipProviderLabel) {
    const providerName = SHIP_CONFIG.source || SHIP_CONFIG.provider;
    els.shipProviderLabel.textContent = `Live vessel tracks via ${providerName}.`;
  }
  const toggle = document.querySelector("[data-ship-toggle]");
  if (toggle) {
    toggle.setAttribute("aria-pressed", "false");
    toggle.addEventListener("click", () => {
      const next = toggle.getAttribute("aria-pressed") !== "true";
      toggle.setAttribute("aria-pressed", next.toString());
      overlay?.setVisible(next);
      if (next) {
        fetchShips(renderer3d, bus, overlay);
      }
    });
  }
};

const setupTileProviders = (renderer3d) => {
  const select = document.getElementById("tile-provider");
  if (!select) return;
  select.innerHTML = "";
  const none = document.createElement("option");
  none.value = "";
  none.textContent = "Base texture";
  select.appendChild(none);

  TILE_CONFIG.order.forEach((id) => {
    const provider = TILE_CONFIG.providers[id];
    if (!provider) return;
    const option = document.createElement("option");
    option.value = id;
    option.textContent = provider.name;
    select.appendChild(option);
  });

  const initial = renderer3d.tileProvider?.id || TILE_CONFIG.activeProvider || "";
  select.value = initial || "";
  renderer3d.setTileProvider(initial || null);

  select.addEventListener("change", () => {
    const value = select.value || null;
    renderer3d.setTileProvider(value);
    if (value) {
      window.localStorage?.setItem?.("c2.tileProvider", value);
    } else {
      window.localStorage?.removeItem?.("c2.tileProvider");
    }
  });
};

const main = () => {
  const bus = new EventBus();
  const world = new World();
  const board = new BoardView(els.board, els.map2d);
  board.resize();

  const renderer3d = new Renderer3D(els.map3d);
  renderer3d.init();

  const edgeLayer = new EdgeLayer(els.edgeLayer, renderer3d, null, (action, entityId) => {
    if (action === "focus") {
      const entity = Number(entityId);
      const geo = world.getComponent(entity, "Geo");
      if (geo) renderer3d.focusOnGeo(geo);
    } else {
      console.info("edge action", { action, entityId });
    }
  });
  edgeLayer.bind();
  const pinLayer = new PinLayer(els.pinLayer, renderer3d, els.board, edgeLayer);
  const flightOverlay = new FlightOverlay(renderer3d, world, els.flightLayer, edgeLayer);
  const satelliteOverlay = new SatelliteOverlay(
    renderer3d,
    world,
    els.satelliteLayer,
    edgeLayer,
  );
  const shipOverlay = new ShipOverlay(renderer3d, world, els.shipLayer, edgeLayer);

  bus.on("entities:update", (payload) => {
    syncEntities(payload, world);
  });
  bus.on("flights:update", (payload) => {
    flightOverlay.ingest(payload);
  });
  bus.on("satellites:update", (payload) => {
    satelliteOverlay.ingest(payload);
  });
  bus.on("ships:update", (payload) => {
    shipOverlay.ingest(payload);
  });

  setupTileProviders(renderer3d);
  setupGlobeControls(renderer3d);
  setupWeatherControls(renderer3d);
  setupFlightControls(renderer3d, bus, flightOverlay);
  setupSatelliteControls(renderer3d, bus, satelliteOverlay);
  setupShipControls(renderer3d, bus, shipOverlay);
  setupPanelCollapsibles();

  const renderLoop = (() => {
    let lastFpsSample = performance.now();
    let lastTick = performance.now();
    let frameCount = 0;
    let fps = 0;

    const tick = () => {
      const now = performance.now();
      const delta = Math.min(64, now - lastTick);
      lastTick = now;
      frameCount += 1;
      if (now - lastFpsSample >= 1000) {
        fps = Math.round((frameCount * 1000) / (now - lastFpsSample));
        frameCount = 0;
        lastFpsSample = now;
      }

      board.drawGrid();

      const entities3d = world
        .query(["Geo", "Renderable"])
        .filter((entity) => {
          const meta = world.getComponent(entity, "Meta");
          if (meta?.kind === "flight" && !flightOverlay.visible) return false;
          if (meta?.kind === "satellite" && !satelliteOverlay.visible) return false;
          if (meta?.kind === "ship" && !shipOverlay.visible) return false;
          return true;
        });
      const shipAltitude = shipBaseAltitude(renderer3d) + altitudeForShip();
      const points = entities3d.map((entity) => {
        const geo = world.getComponent(entity, "Geo");
        const renderable = world.getComponent(entity, "Renderable") || {};
        const meta = world.getComponent(entity, "Meta");
        const altitude =
          meta?.kind === "ship" ? shipAltitude : renderer3d.markerAltitude;
        const pos = renderer3d.positionForGeo(geo, altitude);
        return { ...pos, color: renderable.color };
      });
      renderer3d.setInstances(points);
      renderer3d.render(delta);

      const entitiesPins = world.query(["Geo", "Pin"]);
      pinLayer.syncPins(entitiesPins, world);
      pinLayer.prune(world);
      flightOverlay.sync();
      satelliteOverlay.sync();
      shipOverlay.sync();
      edgeLayer.syncEdges(entities3d, world);
      edgeLayer.prune(world);

      if (els.runtimeStats) {
        els.runtimeStats.textContent = `Entities: ${entities3d.length} · FPS: ${fps}`;
      }
      if (els.cameraStats) {
        if (renderer3d.mode === "iso" && renderer3d.camera?.isOrthographicCamera) {
          els.cameraStats.textContent = `View: Iso · Zoom: ${renderer3d.camera.zoom.toFixed(
            2,
          )}`;
        } else if (renderer3d.camera) {
          const distance = renderer3d.camera.position.length();
          els.cameraStats.textContent = `View: Globe · Dist: ${Math.round(distance)}`;
        }
      }
      if (els.tileStatus) {
        const provider = renderer3d.tileProvider;
        const zoomLabel =
          renderer3d.tileZoom !== null && renderer3d.tileZoom !== undefined
            ? `z${renderer3d.tileZoom}`
            : "--";
        els.tileStatus.textContent = provider
          ? `Tiles: ${provider.name} ${zoomLabel}`
          : "Tiles: disabled";
      }

      requestAnimationFrame(tick);
    };

    return tick;
  })();

  const resize = () => {
    board.resize();
    renderer3d.resize(window.innerWidth, window.innerHeight);
  };

  window.addEventListener("resize", resize);
  resize();

  updateStatus();
  refreshPartials();
  fetchEntities(bus);
  startSse(bus);
  startWs(bus);
  setInterval(updateStatus, 15000);
  setInterval(refreshPartials, 12000);
  setInterval(() => fetchEntities(bus), 20000);
  if (FLIGHT_CONFIG.enabled) {
    setInterval(() => fetchFlights(renderer3d, bus, flightOverlay), FLIGHT_CONFIG.updateIntervalMs);
  }
  if (SATELLITE_CONFIG.enabled) {
    setInterval(
      () => fetchSatellites(renderer3d, bus, satelliteOverlay),
      SATELLITE_CONFIG.updateIntervalMs,
    );
  }
  if (SHIP_CONFIG.enabled) {
    setInterval(() => fetchShips(renderer3d, bus, shipOverlay), SHIP_CONFIG.updateIntervalMs);
  }
  setupDockToggles();
  setupDockControls();
  setupDockDrag();
  setupPillMenus();
  setupWindowMenuActions();
  allDocks().forEach((dock) => {
    setDockState(dock, dock.dataset.state || "open");
  });
  setupLayerToggles();

  requestAnimationFrame(renderLoop);
};

document.addEventListener("DOMContentLoaded", main);

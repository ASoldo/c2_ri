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
  edgeLayer: document.getElementById("edge-layer"),
  dockLeft: document.getElementById("dock-left"),
  dockRight: document.getElementById("dock-right"),
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
    };
  });
  const order = Array.isArray(config.order)
    ? config.order.filter((id) => normalized[id])
    : Object.keys(normalized);
  const saved = window.localStorage?.getItem?.("c2.tileProvider");
  const activeProvider = config.activeProvider || saved || order[0] || null;
  return {
    providers: normalized,
    order,
    activeProvider,
    maxTiles: Number.isFinite(config.maxTiles) ? config.maxTiles : 220,
  };
};

const TILE_CONFIG = buildTileConfig();

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

const tileXForLon = (lon, zoom) => {
  const n = 2 ** zoom;
  const x = Math.floor(((lon + 180) / 360) * n);
  return Math.max(0, Math.min(n - 1, x));
};

const tileYForLat = (lat, zoom) => {
  const n = 2 ** zoom;
  const rad = (clampLat(lat) * Math.PI) / 180;
  const value = (1 - Math.log(Math.tan(rad) + 1 / Math.cos(rad)) / Math.PI) / 2;
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
  const theta = Math.atan2(point.z, point.x);
  const lat = 90 - (phi * 180) / Math.PI;
  const lon = (theta * 180) / Math.PI - 180;
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
    this.group.renderOrder = 2;
    this.scene.add(this.group);
    this.tiles = new Map();
    this.pending = new Set();
    this.provider = null;
    this.maxTiles = TILE_CONFIG.maxTiles;
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
    if (this.loader) {
      this.loader.crossOrigin = this.provider?.proxy ? null : "anonymous";
    }
    this.clear();
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

  pickZoom(camera) {
    if (!this.provider || !camera?.isPerspectiveCamera) return this.provider?.minZoom ?? 0;
    const distance = camera.position.length();
    const ratio = Math.max(
      0,
      Math.min(1, (this.baseDistance * 1.2 - distance) / (this.baseDistance * 0.9)),
    );
    const range = Math.max(0, this.provider.maxZoom - this.provider.minZoom);
    const zoom = Math.round(this.provider.minZoom + range * ratio);
    return Math.min(this.provider.maxZoom, Math.max(this.provider.minZoom, zoom));
  }

  update(camera, size) {
    if (!this.provider || !camera || !size?.width || !size?.height) return;
    const now = performance.now();
    const dir = this.tmpVec.copy(camera.position).normalize();
    const distance = camera.position.length();
    if (
      now - this.lastUpdate < 220 &&
      dir.dot(this.lastDirection) > 0.999 &&
      Math.abs(distance - this.lastDistance) < 0.4
    ) {
      return;
    }
    this.lastUpdate = now;
    this.lastDirection.copy(dir);
    this.lastDistance = distance;
    let zoom = this.pickZoom(camera);
    let tileSet = this.computeVisibleTiles(camera, size, zoom);
    while (tileSet.keys.length > this.maxTiles && zoom > this.provider.minZoom) {
      zoom -= 1;
      tileSet = this.computeVisibleTiles(camera, size, zoom);
    }
    if (zoom !== this.zoom) {
      this.zoom = zoom;
      this.clear();
    }
    const desired = new Set(tileSet.keys);
    for (const key of tileSet.keys) {
      if (this.tiles.has(key) || this.pending.has(key)) continue;
      const tile = tileSet.tiles.get(key);
      if (!tile) continue;
      this.loadTile(tile);
    }
    for (const [key, tile] of this.tiles.entries()) {
      if (!desired.has(key)) {
        tile.mesh?.removeFromParent();
        tile.texture?.dispose();
        tile.geometry?.dispose();
        tile.material?.dispose();
        this.tiles.delete(key);
      }
    }
  }

  computeVisibleTiles(camera, size, zoom) {
    const samples = [
      [-1, -1],
      [1, -1],
      [-1, 1],
      [1, 1],
      [0, 0],
      [0, -0.5],
      [0, 0.5],
      [-0.5, 0],
      [0.5, 0],
    ];
    const geos = samples
      .map(([x, y]) => this.sampleGeo(camera, x, y))
      .filter(Boolean);
    if (!geos.length) {
      return { keys: [], tiles: new Map() };
    }
    const latMin = Math.max(-85, Math.min(...geos.map((g) => g.lat)));
    const latMax = Math.min(85, Math.max(...geos.map((g) => g.lat)));
    const lonStats = this.computeLonRange(geos.map((g) => g.lon));
    const yMin = Math.max(0, tileYForLat(latMax, zoom) - 1);
    const yMax = Math.min(2 ** zoom - 1, tileYForLat(latMin, zoom) + 1);
    const n = 2 ** zoom;
    const xMin = tileXForLon(lonStats.min, zoom);
    const xMax = tileXForLon(lonStats.max, zoom);
    const ranges =
      xMin <= xMax
        ? [[xMin, xMax]]
        : [
            [0, xMax],
            [xMin, n - 1],
          ];
    const tiles = new Map();
    const keys = [];
    ranges.forEach(([start, end]) => {
      for (let x = start - 1; x <= end + 1; x += 1) {
        if (x < 0 || x >= n) continue;
        for (let y = yMin; y <= yMax; y += 1) {
          const key = `${zoom}/${x}/${y}`;
          if (tiles.has(key)) continue;
          const bounds = tileBounds(x, y, zoom);
          tiles.set(key, { key, x, y, zoom, bounds });
          keys.push(key);
        }
      }
    });
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
    return sphereToGeo(this.tmpPoint);
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

  loadTile(tile) {
    if (!this.provider) return;
    const url = this.provider.url
      .replace("{z}", tile.zoom)
      .replace("{x}", tile.x)
      .replace("{y}", tile.y);
    this.pending.add(tile.key);
    this.loader.load(
      url,
      (texture) => {
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.anisotropy = this.renderer?.capabilities?.getMaxAnisotropy?.() || 1;
        const geometry = this.buildTileGeometry(tile.bounds);
        const material = new THREE.MeshStandardMaterial({
          map: texture,
          transparent: true,
          opacity: 0.98,
          roughness: 0.9,
          metalness: 0,
          side: THREE.DoubleSide,
        });
        material.polygonOffset = true;
        material.polygonOffsetFactor = -1;
        material.polygonOffsetUnits = -1;
        material.depthWrite = false;
        const mesh = new THREE.Mesh(geometry, material);
        mesh.renderOrder = 2;
        mesh.frustumCulled = false;
        this.group.add(mesh);
        this.tiles.set(tile.key, {
          mesh,
          texture,
          geometry,
          material,
        });
        this.pending.delete(tile.key);
      },
      undefined,
      () => {
        this.pending.delete(tile.key);
      },
    );
  }

  buildTileGeometry(bounds) {
    const latNorth = bounds.latNorth;
    const latSouth = bounds.latSouth;
    const lonWest = bounds.lonWest;
    const lonEast = bounds.lonEast;
    const phiStart = ((lonWest + 180) * Math.PI) / 180;
    const phiLength = ((lonEast - lonWest) * Math.PI) / 180;
    const thetaStart = ((90 - latNorth) * Math.PI) / 180;
    const thetaLength = ((latNorth - latSouth) * Math.PI) / 180;
    return new THREE.SphereGeometry(this.radius, 12, 12, phiStart, phiLength, thetaStart, thetaLength);
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
    this.globeRotation = Math.PI;
    this.dayMap = null;
    this.nightMap = null;
    this.normalMap = null;
    this.specularMap = null;
    this.cloudsMap = null;
    this.globeMaterial = null;
    this.lightingMode = "day";
    this.showClouds = true;
    this.showAxes = true;
    this.showGrid = true;
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
      }),
    );
    this.clouds.material.depthTest = true;
    this.clouds.renderOrder = 3;
    this.clouds.rotation.y = this.globeRotation;
    this.scene.add(this.clouds);

    this.tileManager = new TileManager(
      this.scene,
      this.globeRadius + 0.6,
      this.renderer,
      this.globeRotation,
    );
    this.tileManager.setBaseDistance(this.defaultDistance);

    const planeMaterial = new THREE.MeshStandardMaterial({
      map: this.dayMap,
      roughness: 0.9,
      metalness: 0.0,
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
    this.scene.add(this.axisHelper);

    this.gridLines = this.buildLatLonGrid(this.globeRadius + 0.6, 20, 10);
    this.scene.add(this.gridLines);

    this.setLightingMode("day");
    this.setCloudsVisible(true);
    this.setAxesVisible(true);
    this.setGridVisible(true);
    this.setTileProvider(TILE_CONFIG.activeProvider);
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
    this.controls.addEventListener("change", () => this.recordCameraTrail());
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

  render() {
    if (!this.renderer || !this.scene || !this.camera) return;
    if (els.map3d && els.map3d.style.display === "none") return;
    if (this.clouds && this.mode === "globe" && this.showClouds) {
      this.clouds.rotation.y += 0.00025;
    }
    this.updateTrails();
    this.updateFocus();
    this.controls?.update();
    if (this.tileManager && this.tileProvider && this.mode === "globe") {
      this.tileManager.update(this.camera, this.size);
      this.tileZoom = this.tileManager.zoom;
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
    return new THREE.LineSegments(geometry, material);
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
    this.popup.innerHTML = `
      <div class="edge-popup-title">${safeLabel}</div>
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
      const symbol = collapseLabel(pin.label) || edgeSymbolFor(meta);
      node.querySelector(".edge-symbol").textContent = symbol;
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
      const state = dock.getAttribute("data-state") || "open";
      dock.setAttribute("data-state", state === "open" ? "closed" : "open");
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

  bus.on("entities:update", (payload) => {
    syncEntities(payload, world);
  });

  setupTileProviders(renderer3d);
  setupGlobeControls(renderer3d);

  const renderLoop = (() => {
    let lastFrame = performance.now();
    let frameCount = 0;
    let fps = 0;

    const tick = () => {
      const now = performance.now();
      frameCount += 1;
      if (now - lastFrame >= 1000) {
        fps = Math.round((frameCount * 1000) / (now - lastFrame));
        frameCount = 0;
        lastFrame = now;
      }

      board.drawGrid();

      const entities3d = world.query(["Geo", "Renderable"]);
      const points = entities3d.map((entity) => {
        const geo = world.getComponent(entity, "Geo");
        const renderable = world.getComponent(entity, "Renderable") || {};
        const pos = renderer3d.positionForGeo(geo, renderer3d.markerAltitude);
        return { ...pos, color: renderable.color };
      });
      renderer3d.setInstances(points);
      renderer3d.render();

      const entitiesPins = world.query(["Geo", "Pin"]);
      pinLayer.syncPins(entitiesPins, world);
      pinLayer.prune(world);
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
  setupDockToggles();
  setupLayerToggles();

  requestAnimationFrame(renderLoop);
};

document.addEventListener("DOMContentLoaded", main);

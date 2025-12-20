import * as THREE from "/static/vendor/three.module.js";
import { OrbitControls } from "/static/vendor/OrbitControls.js";

const els = {
  apiStatus: document.getElementById("api-status"),
  apiDot: document.getElementById("api-dot"),
  streamStatus: document.getElementById("stream-status"),
  wsStatus: document.getElementById("ws-status"),
  runtimeStats: document.getElementById("runtime-stats"),
  cameraStats: document.getElementById("camera-stats"),
  board: document.getElementById("board"),
  layerStack: document.getElementById("layer-stack"),
  map2d: document.getElementById("map-2d"),
  map3d: document.getElementById("map-3d"),
  pinLayer: document.getElementById("pin-layer"),
  dockLeft: document.getElementById("dock-left"),
  dockRight: document.getElementById("dock-right"),
};

const partialEls = Array.from(document.querySelectorAll("[data-partial]"));

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

class Renderer3D {
  constructor(canvas) {
    this.canvas = canvas;
    this.renderer = null;
    this.scene = null;
    this.camera = null;
    this.controls = null;
    this.instances = null;
    this.globe = null;
    this.globeRadius = 120;
    this.tmp = new THREE.Object3D();
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
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(60, 1, 0.1, 2000);
    this.camera.position.set(0, 160, 240);
    this.controls = new OrbitControls(this.camera, this.canvas);
    this.controls.enableDamping = true;
    this.controls.enablePan = true;
    this.controls.target.set(0, 0, 0);

    const hemi = new THREE.HemisphereLight(0xffffff, 0x3f3f3f, 0.9);
    this.scene.add(hemi);

    const texture = new THREE.TextureLoader().load(
      "/static/maps/osm-world-2k.jpg",
    );
    texture.colorSpace = THREE.SRGBColorSpace;

    const globeMaterial = new THREE.MeshStandardMaterial({
      map: texture,
      roughness: 0.8,
      metalness: 0.05,
    });
    this.globe = new THREE.Mesh(
      new THREE.SphereGeometry(this.globeRadius, 64, 64),
      globeMaterial,
    );
    this.globe.rotation.y = Math.PI;
    this.scene.add(this.globe);

    const atmosphere = new THREE.Mesh(
      new THREE.SphereGeometry(this.globeRadius + 2, 64, 64),
      new THREE.MeshBasicMaterial({
        color: 0x7dd3fc,
        transparent: true,
        opacity: 0.08,
      }),
    );
    this.scene.add(atmosphere);

    const geometry = new THREE.SphereGeometry(2.4, 8, 8);
    const material = new THREE.MeshStandardMaterial({
      color: 0xffffff,
      vertexColors: true,
    });
    this.instances = new THREE.InstancedMesh(geometry, material, 1);
    this.scene.add(this.instances);
  }

  resize(width, height) {
    if (!this.renderer || !this.camera) return;
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
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
    this.controls?.update();
    this.renderer.render(this.scene, this.camera);
  }
}

class PinLayer {
  constructor(layerEl, board) {
    this.layerEl = layerEl;
    this.board = board;
    this.nodes = new Map();
  }

  syncPins(entities, world) {
    if (!this.layerEl) return;
    if (this.layerEl.style.display === "none") return;
    entities.forEach((entity) => {
      const pin = world.getComponent(entity, "Pin");
      if (!pin) return;
      let node = this.nodes.get(entity);
      if (!node) {
        node = document.createElement("div");
        node.className = "pin";
        node.textContent = pin.label;
        this.layerEl.appendChild(node);
        this.nodes.set(entity, node);
      } else {
        node.textContent = pin.label;
      }
      const transform = world.getComponent(entity, "Transform");
      if (!transform) return;
      const screen = this.board.worldToScreen(transform);
      node.style.transform = `translate(${screen.x}px, ${screen.y}px)`;
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

const geoToBoard = (geo, scale) => ({
  x: geo.lon * scale,
  y: -geo.lat * scale,
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

const syncEntities = (payload, world, boardScale) => {
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
    world.addComponent(entity, "Transform", geoToBoard(geo, boardScale));
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

const main = () => {
  const bus = new EventBus();
  const world = new World();
  const board = new BoardView(els.board, els.map2d);
  const boardScale = 2.2;

  board.resize();
  board.bindInputs();

  const renderer3d = new Renderer3D(els.map3d);
  renderer3d.init();

  const pinLayer = new PinLayer(els.pinLayer, board);

  bus.on("entities:update", (payload) => {
    syncEntities(payload, world, boardScale);
  });

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
        const pos = geoToSphere(geo, renderer3d.globeRadius + 2.5);
        return { ...pos, color: renderable.color };
      });
      renderer3d.setInstances(points);
      renderer3d.render();

      const entitiesPins = world.query(["Transform", "Pin"]);
      pinLayer.syncPins(entitiesPins, world);
      pinLayer.prune(world);

      if (els.runtimeStats) {
        els.runtimeStats.textContent = `Entities: ${entities3d.length} · FPS: ${fps}`;
      }
      if (els.cameraStats) {
        els.cameraStats.textContent = `Zoom: ${board.zoom.toFixed(1)} · Offset: ${Math.round(
          board.offset.x,
        )},${Math.round(board.offset.y)}`;
      }

      requestAnimationFrame(tick);
    };

    return tick;
  })();

  const resize = () => {
    if (!els.board) return;
    const rect = els.board.getBoundingClientRect();
    board.resize();
    renderer3d.resize(rect.width, rect.height);
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

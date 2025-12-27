import * as THREE from "/static/vendor/three.module.js";
import { OrbitControls } from "/static/vendor/OrbitControls.js";
import {
  MEDIA_OVERLAY_RENDER_ORDER,
  MARKER_ALTITUDE,
  TILE_CONFIG,
  SEA_CONFIG,
  WEATHER_CONFIG,
} from "/static/ui/config.js";
import { ECS_KIND } from "/static/ui/ecs.js";

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

class ParticleField {
  constructor(renderer) {
    this.renderer = renderer;
    this.points = null;
    this.geometry = null;
    this.material = null;
    this.ids = null;
    this.count = 0;
    this.iconTextures = {};
    this.iconReady = {
      flight: 0,
      satellite: 0,
      ship: 0,
    };
    this.kindMask = new Float32Array(8).fill(1.0);
    this.kindMask0 = new THREE.Vector4(1, 1, 1, 1);
    this.kindMask1 = new THREE.Vector4(1, 1, 1, 1);
    this.raycaster = new THREE.Raycaster();
    this.pointer = new THREE.Vector2();
    this.sizeScale = 1.0;
    this.attenuation = 1.0;
  }

  buildFallbackIcon(kind) {
    const canvas = document.createElement("canvas");
    canvas.width = 64;
    canvas.height = 64;
    const ctx = canvas.getContext("2d");
    if (!ctx) return null;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "#ffffff";
    if (kind === "flight") {
      ctx.beginPath();
      ctx.moveTo(32, 4);
      ctx.lineTo(58, 46);
      ctx.lineTo(42, 46);
      ctx.lineTo(32, 60);
      ctx.lineTo(22, 46);
      ctx.lineTo(6, 46);
      ctx.closePath();
      ctx.fill();
    } else if (kind === "satellite") {
      ctx.fillRect(24, 24, 16, 16);
      ctx.fillRect(6, 26, 14, 12);
      ctx.fillRect(44, 26, 14, 12);
      ctx.beginPath();
      ctx.arc(32, 18, 4, 0, Math.PI * 2);
      ctx.fill();
    } else if (kind === "ship") {
      ctx.beginPath();
      ctx.moveTo(12, 36);
      ctx.lineTo(52, 36);
      ctx.lineTo(60, 52);
      ctx.lineTo(4, 52);
      ctx.closePath();
      ctx.fill();
      ctx.fillRect(26, 18, 12, 10);
      ctx.fillRect(18, 30, 28, 6);
    } else {
      ctx.beginPath();
      ctx.arc(32, 32, 20, 0, Math.PI * 2);
      ctx.fill();
    }
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    texture.generateMipmaps = false;
    return texture;
  }

  init() {
    const loader = new THREE.TextureLoader();
    const loadIcon = (key, url) => {
      const title = `${key.charAt(0).toUpperCase()}${key.slice(1)}`;
      const readyKey = `iconReady${title}`;
      const textureKey = `icon${title}`;
      const texture = loader.load(
        url,
        () => {
          this.iconReady[key] = 1;
          if (this.material?.uniforms?.[readyKey]) {
            this.material.uniforms[readyKey].value = 1.0;
          }
          if (this.material?.uniforms?.[textureKey]) {
            this.material.uniforms[textureKey].value = texture;
          }
          this.iconTextures[key] = texture;
        },
        undefined,
        () => {
          if (this.material?.uniforms?.[readyKey]) {
            this.material.uniforms[readyKey].value = 1.0;
          }
        },
      );
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.minFilter = THREE.LinearFilter;
      texture.magFilter = THREE.LinearFilter;
      texture.generateMipmaps = false;
    };
    ["flight", "satellite", "ship"].forEach((key) => {
      const fallback = this.buildFallbackIcon(key);
      if (fallback) {
        this.iconTextures[key] = fallback;
        this.iconReady[key] = 1;
      }
    });
    loadIcon("flight", "/static/assets/plane.png");
    loadIcon("satellite", "/static/assets/satellite.svg");
    loadIcon("ship", "/static/assets/ship.svg");
    const vertexShader = `
      attribute float size;
      attribute vec4 color;
      attribute float kind;
      attribute float heading;
      uniform float sizeScale;
      uniform float sizeAttenuation;
      varying vec4 vColor;
      varying float vKind;
      varying float vHeading;

      void main() {
        vColor = color;
        vKind = kind;
        vHeading = heading;
        vec4 mvPosition = modelViewMatrix * vec4(position, 1.0);
        float attenuate = sizeAttenuation > 0.5 ? (300.0 / -mvPosition.z) : 1.0;
        gl_PointSize = size * sizeScale * attenuate;
        gl_Position = projectionMatrix * mvPosition;
      }
    `;
    const fragmentShader = `
      varying vec4 vColor;
      varying float vKind;
      varying float vHeading;
      uniform vec4 kindMask0;
      uniform vec4 kindMask1;
      uniform sampler2D iconFlight;
      uniform sampler2D iconSatellite;
      uniform sampler2D iconShip;
      uniform float iconReadyFlight;
      uniform float iconReadySatellite;
      uniform float iconReadyShip;

      float kindVisible(float kind) {
        int k = int(kind + 0.5);
        if (k == 0) return kindMask0.x;
        if (k == 1) return kindMask0.y;
        if (k == 2) return kindMask0.z;
        if (k == 3) return kindMask0.w;
        if (k == 4) return kindMask1.x;
        if (k == 5) return kindMask1.y;
        if (k == 6) return kindMask1.z;
        if (k == 7) return kindMask1.w;
        return 1.0;
      }

      vec2 rotateCoord(vec2 coord, float angle) {
        float c = cos(angle);
        float s = sin(angle);
        return vec2(
          coord.x * c - coord.y * s,
          coord.x * s + coord.y * c
        );
      }

      void main() {
        float mask = kindVisible(vKind);
        if (mask < 0.5) discard;
        int k = int(vKind + 0.5);
        if (k == 5 && iconReadyFlight > 0.5) {
          vec2 centered = gl_PointCoord - vec2(0.5);
          vec2 rotated = rotateCoord(centered, -vHeading) + vec2(0.5);
          if (rotated.x < 0.0 || rotated.x > 1.0 || rotated.y < 0.0 || rotated.y > 1.0) discard;
          vec4 tex = texture2D(iconFlight, rotated);
          if (tex.a < 0.05) discard;
          gl_FragColor = vec4(tex.rgb * vColor.rgb, tex.a * vColor.a);
          return;
        }
        if (k == 6 && iconReadySatellite > 0.5) {
          vec4 tex = texture2D(iconSatellite, gl_PointCoord);
          if (tex.a < 0.05) discard;
          gl_FragColor = vec4(tex.rgb * vColor.rgb, tex.a * vColor.a);
          return;
        }
        if (k == 7 && iconReadyShip > 0.5) {
          vec2 centered = gl_PointCoord - vec2(0.5);
          vec2 rotated = rotateCoord(centered, -vHeading) + vec2(0.5);
          if (rotated.x < 0.0 || rotated.x > 1.0 || rotated.y < 0.0 || rotated.y > 1.0) discard;
          vec4 tex = texture2D(iconShip, rotated);
          if (tex.a < 0.05) discard;
          gl_FragColor = vec4(tex.rgb * vColor.rgb, tex.a * vColor.a);
          return;
        }
        vec2 coord = gl_PointCoord - vec2(0.5);
        float dist = length(coord);
        if (dist > 0.5) discard;
        gl_FragColor = vec4(vColor.rgb, vColor.a);
      }
    `;
    this.geometry = new THREE.BufferGeometry();
    this.material = new THREE.ShaderMaterial({
      uniforms: {
        sizeScale: { value: this.sizeScale },
        sizeAttenuation: { value: this.attenuation },
        kindMask0: { value: this.kindMask0 },
        kindMask1: { value: this.kindMask1 },
        iconFlight: { value: this.iconTextures.flight || null },
        iconSatellite: { value: this.iconTextures.satellite || null },
        iconShip: { value: this.iconTextures.ship || null },
        iconReadyFlight: { value: this.iconReady.flight },
        iconReadySatellite: { value: this.iconReady.satellite },
        iconReadyShip: { value: this.iconReady.ship },
      },
      vertexShader,
      fragmentShader,
      transparent: true,
      depthTest: true,
      depthWrite: false,
      side: THREE.DoubleSide,
    });
    this.points = new THREE.Points(this.geometry, this.material);
    this.points.renderOrder = 62;
    this.points.frustumCulled = false;
    if (this.renderer?.scene) {
      this.renderer.scene.add(this.points);
    }
    if (this.raycaster?.params?.Points) {
      this.raycaster.params.Points.threshold = this.renderer?.globeRadius * 0.03;
    }
  }

  setVisible(visible) {
    if (this.points) {
      this.points.visible = visible;
    }
  }

  setKindVisible(kind, visible) {
    if (kind === null || kind === undefined) return;
    const index = Number(kind);
    if (!Number.isFinite(index) || index < 0 || index >= this.kindMask.length) return;
    this.kindMask[index] = visible ? 1.0 : 0.0;
    this.kindMask0.set(
      this.kindMask[0],
      this.kindMask[1],
      this.kindMask[2],
      this.kindMask[3],
    );
    this.kindMask1.set(
      this.kindMask[4],
      this.kindMask[5],
      this.kindMask[6],
      this.kindMask[7],
    );
    if (this.material?.uniforms?.kindMask0) {
      this.material.uniforms.kindMask0.value = this.kindMask0;
    }
    if (this.material?.uniforms?.kindMask1) {
      this.material.uniforms.kindMask1.value = this.kindMask1;
    }
  }

  setSizeScale(value) {
    if (!Number.isFinite(value)) return;
    this.sizeScale = value;
    if (this.material?.uniforms?.sizeScale) {
      this.material.uniforms.sizeScale.value = this.sizeScale;
    }
  }

  setAttenuation(value) {
    const next = value ? 1.0 : 0.0;
    this.attenuation = next;
    if (this.material?.uniforms?.sizeAttenuation) {
      this.material.uniforms.sizeAttenuation.value = next;
    }
  }

  update(renderCache) {
    if (!this.geometry || !this.points) return;
    if (!renderCache || !renderCache.positions || !renderCache.ids) {
      this.points.visible = false;
      return;
    }
    const count = renderCache.ids.length;
    this.points.visible = true;
    this.ids = renderCache.ids;
    this.count = count;
    this.geometry.setDrawRange(0, count);

    this.updateAttribute(
      "position",
      renderCache.positions,
      3,
      THREE.BufferAttribute,
      false,
    );
    if (renderCache.colors) {
      this.updateAttribute(
        "color",
        renderCache.colors,
        4,
        THREE.Uint8BufferAttribute,
        true,
      );
    }
    if (renderCache.sizes) {
      this.updateAttribute(
        "size",
        renderCache.sizes,
        1,
        THREE.BufferAttribute,
        false,
      );
    }
    if (renderCache.kinds) {
      this.updateAttribute(
        "kind",
        renderCache.kinds,
        1,
        THREE.Uint8BufferAttribute,
        false,
      );
    }
    if (renderCache.headings) {
      this.updateAttribute(
        "heading",
        renderCache.headings,
        1,
        THREE.BufferAttribute,
        false,
      );
    }
  }

  updateAttribute(name, array, itemSize, AttributeType, normalized) {
    if (!array) return;
    const current = this.geometry.getAttribute(name);
    if (!current || current.array !== array || current.itemSize !== itemSize) {
      const attribute = new AttributeType(array, itemSize, normalized);
      attribute.setUsage(THREE.DynamicDrawUsage);
      this.geometry.setAttribute(name, attribute);
      return;
    }
    current.needsUpdate = true;
  }

  pick(clientX, clientY) {
    const canvas = this.renderer?.canvas;
    if (!canvas || !this.renderer?.camera || !this.points || !this.ids) return null;
    const rect = canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) return null;
    const x = ((clientX - rect.left) / rect.width) * 2 - 1;
    const y = -((clientY - rect.top) / rect.height) * 2 + 1;
    this.pointer.set(x, y);
    this.raycaster.setFromCamera(this.pointer, this.renderer.camera);
    const hits = this.raycaster.intersectObject(this.points, false);
    if (!hits || !hits.length) return null;
    const index = hits[0].index;
    if (index === undefined || index === null) return null;
    if (index < 0 || index >= this.ids.length) return null;
    return this.ids[index];
  }
}

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
    const desiredOrder = this.provider?.renderOrder ?? 10;
    this.group.renderOrder = Math.min(desiredOrder, MEDIA_OVERLAY_RENDER_ORDER - 5);
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
          typeof this.provider.depthWrite === "boolean"
            ? this.provider.depthWrite
            : !material.transparent && material.depthTest;
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
        const desiredOrder = this.provider.renderOrder ?? 10;
        mesh.renderOrder = Math.min(desiredOrder, MEDIA_OVERLAY_RENDER_ORDER - 5);
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
  constructor(canvas, map2d) {
    this.canvas = canvas;
    this.map2d = map2d;
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
    this.particles = null;
    this.particlesEnabled = true;
    this.globeRadius = 120;
    this.mode = "globe";
    this.size = { width: 1, height: 1 };
    this.planeSize = { width: this.globeRadius * 4, height: this.globeRadius * 2 };
    this.isoFrustum = this.planeSize.height * 1.4;
    this.markerAltitude = MARKER_ALTITUDE;
    this.clouds = null;
    this.axisHelper = null;
    this.gridLines = null;
    this.tileManager = null;
    this.tileProvider = null;
    this.tileZoom = null;
    this.seaManager = null;
    this.seaProvider = null;
    this.seaField = SEA_CONFIG.defaultField;
    this.seaTime = SEA_CONFIG.defaultTime;
    this.seaFormat = SEA_CONFIG.defaultFormat;
    this.seaOpacity = SEA_CONFIG.defaultOpacity;
    this.seaVisible = false;
    this.weatherManager = null;
    this.weatherProvider = null;
    this.weatherField = WEATHER_CONFIG.defaultField;
    this.weatherTime = WEATHER_CONFIG.defaultTime;
    this.weatherFormat = WEATHER_CONFIG.defaultFormat;
    this.weatherOpacity = WEATHER_CONFIG.defaultOpacity;
    this.weatherVisible = false;
    this.globeRotation = Math.PI;
    this.overlayScene = null;
    this.overlayCamera = null;
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
    this.showCameraTrail = false;
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
    this.selectedEntity = null;
    this.selectionRing = null;
  }

  init() {
    if (!this.canvas) return;
    this.renderer = new THREE.WebGLRenderer({
      canvas: this.canvas,
      antialias: true,
      alpha: true,
    });
    this.renderer.setPixelRatio(devicePixelRatio);
    this.renderer.autoClear = false;
    this.renderer.setClearColor(0x000000, 0);
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
      this.globeRadius,
      this.renderer,
      this.globeRotation,
    );
    this.tileManager.setBaseDistance(this.defaultDistance);

    this.seaManager = new TileManager(
      this.scene,
      this.globeRadius,
      this.renderer,
      this.globeRotation,
    );
    this.seaManager.maxTiles = Math.min(SEA_CONFIG.maxTiles, 120);
    this.seaManager.maxCache = Math.max(256, this.seaManager.maxTiles * 3);
    this.seaManager.maxInFlight = SEA_CONFIG.maxInFlight;
    this.seaManager.setBaseDistance(this.defaultDistance);

    this.weatherManager = new TileManager(
      this.scene,
      this.globeRadius,
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

    this.selectionRing = this.buildSelectionRing();
    if (this.selectionRing) {
      this.scene.add(this.selectionRing);
    }

    if (this.particlesEnabled) {
      this.particles = new ParticleField(this);
      this.particles.init();
      this.particles.setKindVisible(ECS_KIND.flight, false);
      this.particles.setKindVisible(ECS_KIND.satellite, false);
      this.particles.setKindVisible(ECS_KIND.ship, false);
    }

    this.setLightingMode("day");
    this.setCloudsVisible(false);
    this.setAxesVisible(true);
    this.setGridVisible(true);
    this.setTileProvider(TILE_CONFIG.activeProvider);
    this.refreshSeaProvider();
    this.setSeaVisible(false);
    this.refreshWeatherProvider();
    this.setWeatherVisible(false);
    this.setMode("globe", true);
  }

  setOverlayScene(scene, camera) {
    this.overlayScene = scene;
    this.overlayCamera = camera;
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
    if (this.particles) {
      this.particles.setVisible(mode === "globe");
      this.particles.setAttenuation(mode === "globe");
    }
    if (this.tileManager) {
      this.tileManager.group.visible = mode === "globe" && Boolean(this.tileProvider);
    }
    if (this.seaManager) {
      this.seaManager.group.visible =
        mode === "globe" && this.seaVisible && Boolean(this.seaProvider);
    }
    if (this.weatherManager) {
      this.weatherManager.group.visible =
        mode === "globe" && this.weatherVisible && Boolean(this.weatherProvider);
    }
    if (this.map2d) {
      this.map2d.style.display = mode === "iso" ? "block" : "none";
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

  setParticles(renderCache) {
    if (!this.particles) return;
    this.particles.update(renderCache);
    this.updateSelection(renderCache);
  }

  setKindVisibility(kind, visible) {
    if (!this.particles) return;
    this.particles.setKindVisible(kind, visible);
  }

  pickEntity(clientX, clientY) {
    if (!this.particles) return null;
    return this.particles.pick(clientX, clientY);
  }

  setSelectedEntity(entityId) {
    this.selectedEntity =
      entityId === null || entityId === undefined ? null : entityId;
    if (this.selectionRing) {
      this.selectionRing.visible = Boolean(this.selectedEntity);
    }
  }

  buildSelectionRing() {
    const inner = this.globeRadius * 0.006;
    const outer = this.globeRadius * 0.012;
    const geometry = new THREE.RingGeometry(inner, outer, 48);
    const material = new THREE.MeshBasicMaterial({
      color: 0xfacc15,
      transparent: true,
      opacity: 0.9,
      depthTest: false,
      depthWrite: false,
      side: THREE.DoubleSide,
    });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.renderOrder = 90;
    mesh.visible = false;
    return mesh;
  }

  updateSelection(renderCache) {
    if (!this.selectionRing || !this.selectedEntity) {
      if (this.selectionRing) this.selectionRing.visible = false;
      return;
    }
    if (!renderCache?.positions || !renderCache?.index) {
      this.selectionRing.visible = false;
      return;
    }
    const key =
      typeof this.selectedEntity === "bigint"
        ? this.selectedEntity
        : BigInt(this.selectedEntity);
    const index = renderCache.index.get(key);
    if (index === undefined) {
      this.selectionRing.visible = false;
      return;
    }
    const positions = renderCache.positions;
    this.selectionRing.position.set(
      positions[index],
      positions[index + 1],
      positions[index + 2],
    );
    const distance = this.camera?.position?.length?.() || this.defaultDistance;
    const ratio = this.defaultDistance ? distance / this.defaultDistance : 1;
    const scale = Math.max(0.6, Math.min(2.4, ratio));
    this.selectionRing.scale.set(scale, scale, scale);
    if (this.camera) {
      this.selectionRing.lookAt(this.camera.position);
    }
    this.selectionRing.visible = this.mode === "globe";
  }

  render(deltaMs = 16, onBeforeOverlay = null) {
    if (!this.renderer || !this.scene || !this.camera) return;
    if (this.canvas && this.canvas.style.display === "none") return;
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
    if (this.seaManager && this.seaProvider && this.seaVisible && this.mode === "globe") {
      this.seaManager.update(this.camera, this.size);
    }
    if (this.weatherManager && this.weatherProvider && this.weatherVisible && this.mode === "globe") {
      this.weatherManager.update(this.camera, this.size);
    }
    this.renderer.clear();
    this.renderer.render(this.scene, this.camera);
    if (this.overlayScene && this.overlayCamera) {
      if (typeof onBeforeOverlay === "function") {
        onBeforeOverlay();
      }
      this.renderer.clearDepth();
      this.renderer.render(this.overlayScene, this.overlayCamera);
    }
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
    if (this.seaManager) {
      this.seaManager.setBaseDistance(distance);
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

  buildSeaProvider() {
    if (!SEA_CONFIG.enabled) return null;
    return {
      id: "sea",
      name: "Sea Overlay",
      url: "/ui/tiles/sea/{z}/{x}/{y}?field={field}&time={time}&format={format}",
      minZoom: SEA_CONFIG.minZoom,
      maxZoom: SEA_CONFIG.maxZoom,
      opacity: this.seaOpacity,
      renderOrder: 40,
      depthTest: false,
      depthWrite: false,
      polygonOffsetFactor: -3,
      polygonOffsetUnits: -3,
      updateIntervalMs: SEA_CONFIG.updateIntervalMs,
      params: {
        field: this.seaField,
        time: this.seaTime,
        format: this.seaFormat,
      },
    };
  }

  refreshSeaProvider() {
    const provider = this.buildSeaProvider();
    this.seaProvider = provider;
    if (this.seaManager) {
      this.seaManager.setProvider(provider);
      if (SEA_CONFIG.maxTiles) {
        this.seaManager.maxTiles = Math.min(SEA_CONFIG.maxTiles, 120);
        this.seaManager.maxCache = Math.max(256, this.seaManager.maxTiles * 3);
      }
      this.seaManager.maxInFlight = SEA_CONFIG.maxInFlight;
      this.seaManager.group.visible =
        this.mode === "globe" && this.seaVisible && Boolean(provider);
    }
  }

  setSeaVisible(visible) {
    this.seaVisible = Boolean(visible);
    if (this.seaManager) {
      this.seaManager.group.visible =
        this.mode === "globe" && this.seaVisible && Boolean(this.seaProvider);
      if (this.seaVisible) {
        this.seaManager.markDirty();
      }
    }
  }

  setSeaField(field) {
    if (!field || field === this.seaField) return;
    this.seaField = field;
    this.refreshSeaProvider();
    if (this.seaManager) {
      this.seaManager.markDirty();
    }
  }

  setSeaTime(time) {
    if (!time || time === this.seaTime) return;
    this.seaTime = time;
    this.refreshSeaProvider();
    if (this.seaManager) {
      this.seaManager.markDirty();
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
      polygonOffsetFactor: -4,
      polygonOffsetUnits: -4,
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
    if (!this.showCameraTrail) return;
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

class MediaOverlay {
  constructor(renderer) {
    this.renderer = renderer;
    this.group = new THREE.Group();
    this.group.renderOrder = MEDIA_OVERLAY_RENDER_ORDER;
    this.rotationY = renderer?.globeRotation || 0;
    this.group.rotation.y = this.rotationY;
    this.group.visible = false;
    this.mesh = null;
    this.material = null;
    this.texture = null;
    this.video = null;
    this.image = null;
    this.hls = null;
    this.staging = null;
    this.enabled = false;
    this.kind = "mjpg";
    this.url = "";
    this.lat = 0;
    this.lon = 0;
    this.widthDeg = 16;
    this.heightDeg = 9;
    this.rotationDeg = 0;
    this.altitude = 0;
    this.scale = 1;
    this.needsFrameUpdate = false;
    this.lastFrameAt = 0;
    this.frameIntervalMs = 33;
    this.audioMuted = true;
    this.playState = "playing";
    this.volume = 0.8;
    if (typeof document !== "undefined") {
      this.staging = document.getElementById("media-overlay-staging");
      if (!this.staging) {
        const div = document.createElement("div");
        div.id = "media-overlay-staging";
        document.body.appendChild(div);
        this.staging = div;
      }
    }
    if (this.renderer?.scene) {
      this.renderer.scene.add(this.group);
    }
  }

  setEnabled(enabled) {
    this.enabled = Boolean(enabled);
    this.group.visible = this.enabled && this.renderer?.mode === "globe";
    if (!this.enabled) {
      this.pauseMedia();
    } else {
      this.resumeMedia();
    }
  }

  setAudioMuted(muted) {
    this.audioMuted = Boolean(muted);
    if (this.video) {
      this.video.muted = this.audioMuted;
      if (!this.audioMuted && Number.isFinite(this.volume)) {
        this.video.volume = this.volume;
      }
    }
  }

  setPlayback(state) {
    const next = state || "playing";
    this.playState = next;
    if (!this.video) return;
    if (next === "playing") {
      if (this.enabled) {
        this.video.play().catch(() => {});
      }
    } else if (next === "paused") {
      this.video.pause();
    } else if (next === "stopped") {
      this.video.pause();
      try {
        this.video.currentTime = 0;
      } catch (err) {
        // Ignore if the media element is not seekable yet.
      }
    }
  }

  setSource(kind, url) {
    const nextKind = kind || "mjpg";
    const nextUrl = url || "";
    if (nextKind === this.kind && nextUrl === this.url) return;
    this.kind = nextKind;
    this.url = nextUrl;
    this.disposeMedia();
    if (!nextUrl) {
      if (this.mesh) this.mesh.visible = false;
      return;
    }
    if (nextKind === "video") {
      const video = document.createElement("video");
      try {
        const parsed = new URL(nextUrl, window.location.href);
        if (parsed.origin !== window.location.origin) {
          video.crossOrigin = "anonymous";
        } else {
          video.removeAttribute("crossorigin");
        }
      } catch (err) {
        video.crossOrigin = "anonymous";
      }
      video.muted = this.audioMuted;
      video.volume = this.volume;
      video.playsInline = true;
      video.loop = true;
      video.autoplay = false;
      video.preload = "auto";
      this.video = video;
      const isHls = nextUrl.toLowerCase().includes(".m3u8");
      const Hls = window.Hls;
      if (isHls && Hls && typeof Hls.isSupported === "function" && Hls.isSupported()) {
        const hls = new Hls({
          enableWorker: true,
          lowLatencyMode: false,
        });
        this.hls = hls;
        hls.loadSource(nextUrl);
        hls.attachMedia(video);
        hls.on(Hls.Events.MANIFEST_PARSED, () => {
          if (this.enabled && this.playState === "playing") {
            video.play().catch(() => {});
          }
        });
        hls.on(Hls.Events.ERROR, (_, data) => {
          if (data?.fatal) {
            console.warn("HLS media error.", data);
          }
        });
      } else {
        video.src = nextUrl;
      }
      const texture = new THREE.VideoTexture(video);
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.minFilter = THREE.LinearFilter;
      texture.magFilter = THREE.LinearFilter;
      texture.generateMipmaps = false;
      this.texture = texture;
      this.needsFrameUpdate = false;
      this.frameIntervalMs = 33;
      if (this.enabled && this.playState === "playing") {
        video.play().catch(() => {});
      }
    } else {
      const image = new Image();
      image.crossOrigin = "anonymous";
      image.referrerPolicy = "no-referrer";
      image.decoding = "async";
      image.src = nextUrl;
      this.image = image;
      if (this.staging && !this.staging.contains(image)) {
        this.staging.appendChild(image);
      }
      const texture = new THREE.Texture(image);
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.minFilter = THREE.LinearFilter;
      texture.magFilter = THREE.LinearFilter;
      texture.generateMipmaps = false;
      texture.needsUpdate = false;
      image.onload = () => {
        texture.needsUpdate = true;
        this.lastFrameAt = performance.now();
      };
      image.onerror = () => {
        console.warn("Media overlay failed to load source.", {
          kind: nextKind,
          url: nextUrl,
        });
      };
      this.texture = texture;
      if (nextKind === "gif" || nextKind === "mjpg" || nextKind === "rtsp") {
        this.needsFrameUpdate = true;
        this.frameIntervalMs = nextKind === "gif" ? 66 : 33;
      } else {
        this.needsFrameUpdate = false;
        this.frameIntervalMs = 33;
      }
    }
    this.ensureMesh();
    this.updateMaterial();
    this.rebuildGeometry();
    if (this.mesh) this.mesh.visible = true;
  }

  setTransform(config) {
    if (!config) return;
    if (Number.isFinite(config.lat)) this.lat = config.lat;
    if (Number.isFinite(config.lon)) this.lon = config.lon;
    if (Number.isFinite(config.widthDeg)) this.widthDeg = config.widthDeg;
    if (Number.isFinite(config.heightDeg)) this.heightDeg = config.heightDeg;
    if (Number.isFinite(config.rotationDeg)) this.rotationDeg = config.rotationDeg;
    if (Number.isFinite(config.altitude)) this.altitude = config.altitude;
    if (Number.isFinite(config.scale)) this.scale = config.scale;
    this.rebuildGeometry();
  }

  update(now = performance.now()) {
    if (!this.enabled) return;
    this.group.visible = this.renderer?.mode === "globe";
    if (!this.group.visible) return;
    if (!this.needsFrameUpdate || !this.texture || !this.image) return;
    const hasFrame =
      this.image.complete &&
      ((this.image.naturalWidth && this.image.naturalHeight) ||
        (this.image.width && this.image.height));
    if (!hasFrame) return;
    if (this.lastFrameAt === 0 || now - this.lastFrameAt > this.frameIntervalMs) {
      this.texture.needsUpdate = true;
      this.lastFrameAt = now;
    }
  }

  clear() {
    this.setSource(this.kind, "");
    if (this.mesh) {
      this.mesh.visible = false;
    }
    this.setPlayback("stopped");
  }

  pauseMedia() {
    if (this.video) {
      this.video.pause();
    }
  }

  resumeMedia() {
    if (this.video) {
      if (this.playState === "playing") {
        this.video.play().catch(() => {});
      }
    }
  }

  disposeMedia() {
    if (this.hls) {
      this.hls.destroy();
    }
    this.hls = null;
    if (this.video) {
      this.video.pause();
      this.video.removeAttribute("src");
      this.video.load();
    }
    this.video = null;
    if (this.image) {
      if (this.image.parentNode) {
        this.image.parentNode.removeChild(this.image);
      }
      this.image.onload = null;
      this.image.onerror = null;
      this.image.src = "";
    }
    this.image = null;
    if (this.texture) {
      this.texture.dispose();
    }
    this.texture = null;
    this.needsFrameUpdate = false;
    this.lastFrameAt = 0;
    this.frameIntervalMs = 33;
    if (this.material) {
      this.material.map = null;
      this.material.needsUpdate = true;
    }
  }

  ensureMesh() {
    if (this.mesh) return;
    const material = new THREE.MeshBasicMaterial({
      transparent: true,
      opacity: 1,
      color: new THREE.Color(0xffffff),
      side: THREE.FrontSide,
      depthTest: false,
      depthWrite: false,
    });
    material.alphaTest = 0.01;
    material.polygonOffset = false;
    material.polygonOffsetFactor = 0;
    material.polygonOffsetUnits = 0;
    const geometry = new THREE.SphereGeometry(
      (this.renderer?.globeRadius || 120) + this.altitude,
      32,
      24,
    );
    const mesh = new THREE.Mesh(geometry, material);
    mesh.renderOrder = MEDIA_OVERLAY_RENDER_ORDER;
    mesh.frustumCulled = true;
    this.mesh = mesh;
    this.material = material;
    this.group.add(mesh);
  }

  updateMaterial() {
    if (!this.material || !this.texture) return;
    this.material.map = this.texture;
    this.material.needsUpdate = true;
  }

  rebuildGeometry() {
    if (!this.mesh || !this.renderer) return;
    const radius =
      (this.renderer?.globeRadius || 120) +
      Math.max(0, this.altitude);
    const width = Math.max(0.5, Math.abs(this.widthDeg) * Math.max(0.1, this.scale));
    const height = Math.max(0.5, Math.abs(this.heightDeg) * Math.max(0.1, this.scale));
    const widthRad = THREE.MathUtils.degToRad(width);
    const heightRad = THREE.MathUtils.degToRad(height);
    const widthSegments = Math.min(96, Math.max(10, Math.round(width / 1.8)));
    const heightSegments = Math.min(72, Math.max(8, Math.round(height / 1.8)));
    const lat = clampLat(this.lat);
    const lon = wrapLon(this.lon);
    const thetaCenter = THREE.MathUtils.degToRad(90 - lat);
    const phiCenter = THREE.MathUtils.degToRad(lon + 180);
    const sinTheta = Math.sin(thetaCenter);
    const cosTheta = Math.cos(thetaCenter);
    const sinPhi = Math.sin(phiCenter);
    const cosPhi = Math.cos(phiCenter);
    const normal = new THREE.Vector3(
      -cosPhi * sinTheta,
      cosTheta,
      sinPhi * sinTheta,
    ).normalize();
    const east = new THREE.Vector3(sinPhi, 0, cosPhi);
    if (east.lengthSq() < 1e-8) {
      east.set(1, 0, 0);
    } else {
      east.normalize();
    }
    const north = new THREE.Vector3().crossVectors(normal, east).normalize();
    const columns = widthSegments + 1;
    const rows = heightSegments + 1;
    const positions = new Float32Array(columns * rows * 3);
    const uvs = new Float32Array(columns * rows * 2);
    let posIndex = 0;
    let uvIndex = 0;
    for (let row = 0; row < rows; row += 1) {
      const v = row / heightSegments;
      const vAngle = (0.5 - v) * heightRad;
      const cosV = Math.cos(vAngle);
      const sinV = Math.sin(vAngle);
      for (let col = 0; col < columns; col += 1) {
        const u = col / widthSegments;
        const uAngle = (u - 0.5) * widthRad;
        const cosU = Math.cos(uAngle);
        const sinU = Math.sin(uAngle);
        const dirX = normal.x * cosV * cosU + east.x * cosV * sinU + north.x * sinV;
        const dirY = normal.y * cosV * cosU + east.y * cosV * sinU + north.y * sinV;
        const dirZ = normal.z * cosV * cosU + east.z * cosV * sinU + north.z * sinV;
        positions[posIndex++] = dirX * radius;
        positions[posIndex++] = dirY * radius;
        positions[posIndex++] = dirZ * radius;
        uvs[uvIndex++] = u;
        uvs[uvIndex++] = 1 - v;
      }
    }
    const indices = new Uint16Array(widthSegments * heightSegments * 6);
    let index = 0;
    for (let row = 0; row < heightSegments; row += 1) {
      for (let col = 0; col < widthSegments; col += 1) {
        const a = row * columns + col;
        const b = a + columns;
        const c = b + 1;
        const d = a + 1;
        indices[index++] = a;
        indices[index++] = b;
        indices[index++] = d;
        indices[index++] = b;
        indices[index++] = c;
        indices[index++] = d;
      }
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setIndex(new THREE.BufferAttribute(indices, 1));
    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute("uv", new THREE.BufferAttribute(uvs, 2));
    geometry.computeBoundingSphere();
    this.mesh.geometry.dispose();
    this.mesh.geometry = geometry;
    this.mesh.rotation.set(0, 0, 0);
    if (Number.isFinite(this.rotationDeg) && this.rotationDeg !== 0) {
      this.mesh.rotateOnAxis(
        normal,
        -THREE.MathUtils.degToRad(this.rotationDeg),
      );
    }
  }
}


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

const wrapLon = (lon) => {
  const value = ((lon + 180) % 360 + 360) % 360 - 180;
  return value;
};

export { Renderer3D, MediaOverlay };

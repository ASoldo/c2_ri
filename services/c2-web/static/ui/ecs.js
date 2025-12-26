const ECS_WASM_URL = "/static/wasm/c2-ecs-wasm.wasm";

const dispatchEcsEvent = (name, detail) => {
  window.dispatchEvent(new CustomEvent(name, { detail }));
};

const loadWasmModule = async (url, imports = {}) => {
  if ("instantiateStreaming" in WebAssembly) {
    try {
      return await WebAssembly.instantiateStreaming(fetch(url), imports);
    } catch (error) {
      console.warn("ECS streaming WASM failed, falling back to ArrayBuffer.", error);
    }
  }
  const response = await fetch(url);
  const bytes = await response.arrayBuffer();
  return WebAssembly.instantiate(bytes, imports);
};

export const ECS_KIND = {
  unknown: 0,
  asset: 1,
  unit: 2,
  mission: 3,
  incident: 4,
  flight: 5,
  satellite: 6,
  ship: 7,
};

export const ecsKindForType = (kind) => {
  switch (kind) {
    case "asset":
      return ECS_KIND.asset;
    case "unit":
      return ECS_KIND.unit;
    case "mission":
      return ECS_KIND.mission;
    case "incident":
      return ECS_KIND.incident;
    case "flight":
      return ECS_KIND.flight;
    case "satellite":
      return ECS_KIND.satellite;
    case "ship":
      return ECS_KIND.ship;
    default:
      return ECS_KIND.unknown;
  }
};

const hashString = (value) => {
  let hash = 2166136261;
  for (let i = 0; i < value.length; i += 1) {
    hash ^= value.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
};

export const hashToGeo = (id) => {
  const h1 = hashString(id);
  const h2 = hashString(`${id}:lon`);
  const lat = (h1 / 0xffffffff) * 180 - 90;
  const lon = (h2 / 0xffffffff) * 360 - 180;
  return { lat, lon };
};

export const ecsIdForKey = (key) => {
  const hi = hashString(key);
  const lo = hashString(`${key}:ecs`);
  return (BigInt(hi) << 32n) | BigInt(lo);
};

export const parseEntityId = (value) => {
  if (typeof value === "bigint") return value;
  if (typeof value === "number" && Number.isFinite(value)) {
    return BigInt(Math.trunc(value));
  }
  if (typeof value === "string" && value.length) {
    try {
      return BigInt(value);
    } catch (error) {
      return null;
    }
  }
  return null;
};

export const formatEntityId = (value) => {
  if (typeof value === "bigint") return value.toString();
  if (value === null || value === undefined) return "";
  return String(value);
};

export const ecsRuntime = {
  ready: false,
  instance: null,
  memory: null,
  initPromise: null,
  renderCache: {
    ids: null,
    positions: null,
    colors: null,
    sizes: null,
    kinds: null,
    index: new Map(),
  },
  kindCache: new Map(),
  async init() {
    if (this.initPromise) return this.initPromise;
    this.initPromise = (async () => {
      dispatchEcsEvent("ecs-loading");
      const result = await loadWasmModule(ECS_WASM_URL, {});
      this.instance = result.instance;
      this.memory = this.instance.exports?.memory || null;
      if (this.instance.exports?.ecs_init) {
        this.instance.exports.ecs_init();
      }
      this.ready = true;
      dispatchEcsEvent("ecs-ready");
      return this;
    })().catch((error) => {
      console.error("ECS WASM init failed", error);
      dispatchEcsEvent("ecs-error", { message: error?.message || String(error) });
      throw error;
    });
    return this.initPromise;
  },
  tick() {
    if (!this.ready || !this.instance?.exports?.ecs_tick) return;
    this.instance.exports.ecs_tick();
  },
  setGlobeRadius(radius) {
    if (!this.ready || !this.instance?.exports?.ecs_set_globe_radius) return;
    if (!Number.isFinite(radius)) return;
    this.instance.exports.ecs_set_globe_radius(radius);
  },
  ingestBatch(items) {
    if (!this.ready || !this.memory) return false;
    const exports = this.instance.exports;
    if (!exports?.ecs_ingest_reserve || !exports?.ecs_ingest_ids_ptr) return false;
    const count = items.length;
    if (!count) return true;
    exports.ecs_ingest_reserve(count);
    const idsPtr = exports.ecs_ingest_ids_ptr();
    const geosPtr = exports.ecs_ingest_geos_ptr();
    const kindsPtr = exports.ecs_ingest_kinds_ptr
      ? exports.ecs_ingest_kinds_ptr()
      : 0;
    const altsPtr = exports.ecs_ingest_alts_ptr
      ? exports.ecs_ingest_alts_ptr()
      : 0;
    const sizesPtr = exports.ecs_ingest_sizes_ptr
      ? exports.ecs_ingest_sizes_ptr()
      : 0;
    const colorsPtr = exports.ecs_ingest_colors_ptr
      ? exports.ecs_ingest_colors_ptr()
      : 0;
    const ids = new BigUint64Array(this.memory.buffer, idsPtr, count);
    const geos = new Float32Array(this.memory.buffer, geosPtr, count * 2);
    const kinds = kindsPtr
      ? new Uint8Array(this.memory.buffer, kindsPtr, count)
      : null;
    const alts = altsPtr
      ? new Float32Array(this.memory.buffer, altsPtr, count)
      : null;
    const sizes = sizesPtr
      ? new Float32Array(this.memory.buffer, sizesPtr, count)
      : null;
    const colors = colorsPtr
      ? new Uint8Array(this.memory.buffer, colorsPtr, count * 4)
      : null;
    items.forEach((item, index) => {
      const id = typeof item.id === "bigint" ? item.id : BigInt(item.id);
      ids[index] = id;
      const offset = index * 2;
      geos[offset] = item.lat;
      geos[offset + 1] = item.lon;
      if (kinds) {
        kinds[index] =
          typeof item.kind === "number" ? item.kind : ECS_KIND.unknown;
      }
      if (alts) {
        alts[index] = Number.isFinite(item.altitude) ? item.altitude : 0;
      }
      if (sizes) {
        sizes[index] = Number.isFinite(item.size) ? item.size : 6.0;
      }
      if (colors) {
        const cOffset = index * 4;
        const color = Array.isArray(item.color)
          ? item.color
          : [0x38, 0xbd, 0xf8, 0xff];
        colors[cOffset] = color[0] ?? 0x38;
        colors[cOffset + 1] = color[1] ?? 0xbd;
        colors[cOffset + 2] = color[2] ?? 0xf8;
        colors[cOffset + 3] = color[3] ?? 0xff;
      }
    });
    exports.ecs_ingest_commit(count);
    return true;
  },
  upsertEntity(id, lat, lon, kind = ECS_KIND.unknown, style = null) {
    if (!this.ready || !this.instance?.exports) return;
    const ecsId = typeof id === "bigint" ? id : BigInt(id);
    if (this.instance.exports.ecs_upsert_entity_style && style) {
      const color = Array.isArray(style.color) ? style.color : [0x38, 0xbd, 0xf8, 0xff];
      const altitude = Number.isFinite(style.altitude) ? style.altitude : 0;
      const size = Number.isFinite(style.size) ? style.size : 6.0;
      this.instance.exports.ecs_upsert_entity_style(
        ecsId,
        lat,
        lon,
        kind,
        altitude,
        size,
        color[0] ?? 0x38,
        color[1] ?? 0xbd,
        color[2] ?? 0xf8,
        color[3] ?? 0xff,
      );
    } else if (this.instance.exports.ecs_upsert_entity_kind) {
      this.instance.exports.ecs_upsert_entity_kind(ecsId, lat, lon, kind);
    } else if (this.instance.exports.ecs_upsert_entity) {
      this.instance.exports.ecs_upsert_entity(ecsId, lat, lon);
    }
  },
  removeEntity(id) {
    if (!this.ready || !this.instance?.exports?.ecs_remove_entity) return;
    const ecsId = typeof id === "bigint" ? id : BigInt(id);
    this.instance.exports.ecs_remove_entity(ecsId);
  },
  entityCount() {
    if (!this.ready || !this.instance?.exports?.ecs_entity_count) return 0;
    return this.instance.exports.ecs_entity_count();
  },
  readRenderBuffers() {
    if (!this.ready || !this.memory) return null;
    const exports = this.instance.exports;
    if (!exports?.ecs_ids_ptr || !exports?.ecs_positions_ptr) return null;
    const idsPtr = exports.ecs_ids_ptr();
    const idsLen = exports.ecs_ids_len();
    const posPtr = exports.ecs_positions_ptr();
    const posLen = exports.ecs_positions_len();
    const colorsPtr = exports.ecs_colors_ptr ? exports.ecs_colors_ptr() : 0;
    const colorsLen = exports.ecs_colors_len ? exports.ecs_colors_len() : 0;
    const sizesPtr = exports.ecs_sizes_ptr ? exports.ecs_sizes_ptr() : 0;
    const sizesLen = exports.ecs_sizes_len ? exports.ecs_sizes_len() : 0;
    const kindsPtr = exports.ecs_kinds_ptr ? exports.ecs_kinds_ptr() : 0;
    const kindsLen = exports.ecs_kinds_len ? exports.ecs_kinds_len() : 0;
    const ids =
      this.renderCache.ids &&
      this.renderCache.ids.byteOffset === idsPtr &&
      this.renderCache.ids.length === idsLen
        ? this.renderCache.ids
        : new BigUint64Array(this.memory.buffer, idsPtr, idsLen);
    const positions =
      this.renderCache.positions &&
      this.renderCache.positions.byteOffset === posPtr &&
      this.renderCache.positions.length === posLen
        ? this.renderCache.positions
        : new Float32Array(this.memory.buffer, posPtr, posLen);
    const colors = colorsPtr && colorsLen
      ? this.renderCache.colors &&
        this.renderCache.colors.byteOffset === colorsPtr &&
        this.renderCache.colors.length === colorsLen
        ? this.renderCache.colors
        : new Uint8Array(this.memory.buffer, colorsPtr, colorsLen)
      : null;
    const sizes = sizesPtr && sizesLen
      ? this.renderCache.sizes &&
        this.renderCache.sizes.byteOffset === sizesPtr &&
        this.renderCache.sizes.length === sizesLen
        ? this.renderCache.sizes
        : new Float32Array(this.memory.buffer, sizesPtr, sizesLen)
      : null;
    const kinds = kindsPtr && kindsLen
      ? this.renderCache.kinds &&
        this.renderCache.kinds.byteOffset === kindsPtr &&
        this.renderCache.kinds.length === kindsLen
        ? this.renderCache.kinds
        : new Uint8Array(this.memory.buffer, kindsPtr, kindsLen)
      : null;
    return { ids, positions, colors, sizes, kinds };
  },
  refreshRenderCache() {
    const data = this.readRenderBuffers();
    if (!data) {
      this.renderCache.ids = null;
      this.renderCache.positions = null;
      this.renderCache.colors = null;
      this.renderCache.sizes = null;
      this.renderCache.kinds = null;
      this.renderCache.index.clear();
      this.kindCache.clear();
      return null;
    }
    this.renderCache.ids = data.ids;
    this.renderCache.positions = data.positions;
    this.renderCache.colors = data.colors;
    this.renderCache.sizes = data.sizes;
    this.renderCache.kinds = data.kinds;
    this.renderCache.index.clear();
    for (let i = 0; i < data.ids.length; i += 1) {
      this.renderCache.index.set(data.ids[i], i * 3);
    }
    return this.renderCache;
  },
  readKindIds(kind) {
    if (!this.ready || !this.memory || !this.instance?.exports) return null;
    const exports = this.instance.exports;
    if (!exports.ecs_kind_ids_ptr || !exports.ecs_kind_ids_len) return null;
    const ptr = exports.ecs_kind_ids_ptr(kind);
    const len = exports.ecs_kind_ids_len(kind);
    if (!ptr || !len) return new BigUint64Array();
    return new BigUint64Array(this.memory.buffer, ptr, len);
  },
  refreshKindCache(kinds) {
    this.kindCache.clear();
    if (!Array.isArray(kinds)) return this.kindCache;
    kinds.forEach((kind) => {
      const ids = this.readKindIds(kind);
      if (ids) {
        this.kindCache.set(kind, ids);
      }
    });
    return this.kindCache;
  },
  positionForId(id, altitude, globeRadius) {
    if (!this.renderCache.positions) return null;
    const key = typeof id === "bigint" ? id : BigInt(id);
    const index = this.renderCache.index.get(key);
    if (index === undefined) return null;
    const positions = this.renderCache.positions;
    const baseX = positions[index];
    const baseY = positions[index + 1];
    const baseZ = positions[index + 2];
    if (!Number.isFinite(altitude) || !Number.isFinite(globeRadius)) {
      return { x: baseX, y: baseY, z: baseZ };
    }
    const scale = (globeRadius + altitude) / globeRadius;
    return { x: baseX * scale, y: baseY * scale, z: baseZ * scale };
  },
};

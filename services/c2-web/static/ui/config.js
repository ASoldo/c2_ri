export const MEDIA_OVERLAY_RENDER_ORDER = 55;
export const MARKER_ALTITUDE = 3.0;
export const SHIP_BASE_ALTITUDE = 0.6;
export const PARTICLE_SIZES = {
  default: 6.0,
  asset: 6.5,
  unit: 6.5,
  mission: 6.5,
  incident: 6.5,
  flight: 13.5,
  satellite: 11.0,
  ship: 12.5,
};
export const LABEL_LOD_MAX = 2200;

export const DEFAULT_TILE_PROVIDERS = {
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
    const renderOrder = Number.isFinite(provider.renderOrder)
      ? provider.renderOrder
      : 10;
    const polygonOffsetFactor = Number.isFinite(provider.polygonOffsetFactor)
      ? provider.polygonOffsetFactor
      : -2;
    const polygonOffsetUnits = Number.isFinite(provider.polygonOffsetUnits)
      ? provider.polygonOffsetUnits
      : -2;
    normalized[id] = {
      id,
      name: provider.name || id,
      url: resolvedUrl,
      remoteUrl,
      proxy,
      minZoom: Number.isFinite(provider.minZoom) ? provider.minZoom : 0,
      maxZoom: Number.isFinite(provider.maxZoom) ? provider.maxZoom : 19,
      zoomBias: Number.isFinite(provider.zoomBias) ? provider.zoomBias : 0,
      opacity: Number.isFinite(provider.opacity) ? provider.opacity : undefined,
      transparent: provider.transparent === true,
      renderOrder,
      polygonOffsetFactor,
      polygonOffsetUnits,
      depthTest: typeof provider.depthTest === "boolean" ? provider.depthTest : true,
      depthWrite:
        typeof provider.depthWrite === "boolean" ? provider.depthWrite : undefined,
      updateIntervalMs: Number.isFinite(provider.updateIntervalMs)
        ? provider.updateIntervalMs
        : undefined,
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

export const TILE_CONFIG = buildTileConfig();

export const DEFAULT_WEATHER_FIELDS = [
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
  const enabled =
    config.enabled !== undefined ? Boolean(config.enabled) : true;
  return {
    enabled,
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

export const WEATHER_CONFIG = buildWeatherConfig();

export const DEFAULT_SEA_FIELDS = [
  "OSCAR_Sea_Surface_Currents_Zonal",
  "OSCAR_Sea_Surface_Currents_Meridional",
  "AMSRU_Ocean_Wind_Speed_Day",
  "JPL_MEaSUREs_L4_Sea_Surface_Height_Anomalies",
];

const buildSeaConfig = () => {
  const config = window.C2_SEA_CONFIG || {};
  let fields = Array.isArray(config.fields) && config.fields.length
    ? config.fields.filter(Boolean)
    : DEFAULT_SEA_FIELDS.slice();
  if (!fields.length) fields = DEFAULT_SEA_FIELDS.slice();
  const defaultField = fields.includes(config.defaultField)
    ? config.defaultField
    : fields[0];
  const defaultTime = config.defaultTime || "default";
  const defaultFormat = config.defaultFormat || "png";
  const defaultOpacity = Number.isFinite(config.defaultOpacity)
    ? config.defaultOpacity
    : 0.45;
  const maxTiles = Number.isFinite(config.maxTiles) ? config.maxTiles : 50;
  const updateIntervalMs = Number.isFinite(config.updateIntervalMs)
    ? config.updateIntervalMs
    : 1100;
  const maxInFlight = Number.isFinite(config.maxInFlight) ? config.maxInFlight : 2;
  const minZoom = Number.isFinite(config.minZoom) ? config.minZoom : 0;
  const maxZoom = Number.isFinite(config.maxZoom) ? config.maxZoom : 6;
  const enabled =
    config.enabled !== undefined ? Boolean(config.enabled) : true;
  return {
    enabled,
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

export const SEA_CONFIG = buildSeaConfig();

export const DEFAULT_FLIGHT_CONFIG = {
  enabled: true,
  provider: "adsb_lol",
  updateIntervalMs: 5000,
  minIntervalMs: 3500,
  maxFlights: 80,
  trailPoints: 24,
  trailMaxAgeMs: 240000,
  spanMinDeg: 8,
  spanMaxDeg: 60,
  altitudeScale: 0.08,
  source: "ADSB.lol",
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

export const FLIGHT_CONFIG = buildFlightConfig();

export const DEFAULT_SATELLITE_CONFIG = {
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

export const SATELLITE_CONFIG = buildSatelliteConfig();

export const DEFAULT_SHIP_CONFIG = {
  enabled: true,
  provider: "aishub",
  updateIntervalMs: 9000,
  maxShips: 200,
  spanMinDeg: 6,
  spanMaxDeg: 70,
  altitude: 0.12,
  source: "AISHub",
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

export const SHIP_CONFIG = buildShipConfig();

export const clampLat = (lat) => Math.max(-85.05112878, Math.min(85.05112878, lat));

export const wrapLon = (lon) => {
  const value = ((lon + 180) % 360 + 360) % 360 - 180;
  return value;
};

export const forEachEntity = (entities, callback) => {
  if (!entities || !callback) return;
  if (Array.isArray(entities)) {
    entities.forEach((entry) => forEachEntity(entry, callback));
    return;
  }
  if (ArrayBuffer.isView(entities)) {
    for (let i = 0; i < entities.length; i += 1) {
      callback(entities[i]);
    }
    return;
  }
  if (typeof entities.forEach === "function") {
    entities.forEach(callback);
  }
};

export const parseNumber = (value, fallback) => {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : fallback;
};

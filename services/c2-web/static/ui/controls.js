import {
  BUBBLE_LABELS_ENABLED,
  FLIGHT_CONFIG,
  DEFAULT_SEA_FIELDS,
  SEA_CONFIG,
  SATELLITE_CONFIG,
  SHIP_CONFIG,
  TILE_CONFIG,
  WEATHER_CONFIG,
} from "/static/ui/config.js";
import { ECS_KIND } from "/static/ui/ecs.js";
import { clampLat, parseNumber, wrapLon } from "/static/ui/utils.js";
import { els } from "/static/ui/dom.js";

export const setupLayerToggles = (renderer3d, bubbleOverlay) => {
  document.querySelectorAll("[data-layer-toggle]").forEach((button) => {
    const layerName = button.dataset.layerToggle;
    if (!layerName) return;
    const initial = true;
    button.dataset.active = initial ? "true" : "false";
    if (layerName === "pins") {
      renderer3d?.setKindVisibility?.(ECS_KIND.asset, initial);
      renderer3d?.setKindVisibility?.(ECS_KIND.unit, initial);
      renderer3d?.setKindVisibility?.(ECS_KIND.mission, initial);
      renderer3d?.setKindVisibility?.(ECS_KIND.incident, initial);
      if (BUBBLE_LABELS_ENABLED) {
        bubbleOverlay?.setPinsVisible?.(initial);
      }
    }
    button.addEventListener("click", () => {
      const next = button.dataset.active !== "true";
      button.dataset.active = next ? "true" : "false";
      if (layerName === "pins") {
        renderer3d?.setKindVisibility?.(ECS_KIND.asset, next);
        renderer3d?.setKindVisibility?.(ECS_KIND.unit, next);
        renderer3d?.setKindVisibility?.(ECS_KIND.mission, next);
        renderer3d?.setKindVisibility?.(ECS_KIND.incident, next);
        if (BUBBLE_LABELS_ENABLED) {
          bubbleOverlay?.setPinsVisible?.(next);
        }
      }
    });
  });
};

export const setupGlobeControls = (renderer3d) => {
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

const formatOverlayLabel = (value) => {
  if (!value) return "";
  return value
    .replace(/_/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/^./, (char) => char.toUpperCase());
};

export const setupWeatherControls = (renderer3d) => {
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
      option.textContent = formatOverlayLabel(field);
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

export const setupSeaControls = (renderer3d) => {
  const panel = document.getElementById("sea-panel");
  if (!SEA_CONFIG.enabled) {
    if (panel) panel.style.display = "none";
    return;
  }
  if (panel) panel.style.display = "block";
  const select = document.getElementById("sea-field");
  if (select) {
    select.innerHTML = "";
    const fields =
      Array.isArray(SEA_CONFIG.fields) && SEA_CONFIG.fields.length
        ? SEA_CONFIG.fields
        : DEFAULT_SEA_FIELDS;
    fields.forEach((field) => {
      const option = document.createElement("option");
      option.value = field;
      option.textContent = formatOverlayLabel(field);
      select.appendChild(option);
    });
    const initial = SEA_CONFIG.defaultField || fields[0];
    if (initial) {
      select.value = initial;
      renderer3d.setSeaField(initial);
    }
    select.addEventListener("change", () => {
      renderer3d.setSeaField(select.value);
    });
  }
  const toggle = document.querySelector("[data-sea-toggle]");
  if (toggle) {
    toggle.setAttribute("aria-pressed", "false");
    toggle.addEventListener("click", () => {
      const next = toggle.getAttribute("aria-pressed") !== "true";
      toggle.setAttribute("aria-pressed", next.toString());
      renderer3d.setSeaVisible(next);
    });
  }
};

const clampFlightLat = (lat) => Math.max(-85, Math.min(85, lat));
const clampFlightLon = (lon) => Math.max(-180, Math.min(180, lon));
const clampShipLat = (lat) => Math.max(-85, Math.min(85, lat));
const clampShipLon = (lon) => Math.max(-180, Math.min(180, lon));

const computeFlightBounds = (renderer3d) => {
  if (!renderer3d || renderer3d.mode !== "globe") return null;
  if (FLIGHT_CONFIG.global) {
    return {
      lamin: clampFlightLat(-85),
      lamax: clampFlightLat(85),
      lomin: clampFlightLon(-180),
      lomax: clampFlightLon(180),
    };
  }
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
  if (SHIP_CONFIG.global) {
    return {
      lamin: clampShipLat(-85),
      lamax: clampShipLat(85),
      lomin: clampShipLon(-180),
      lomax: clampShipLon(180),
    };
  }
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

export const fetchFlights = async (renderer3d, bus, overlay) => {
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

export const fetchSatellites = async (renderer3d, bus, overlay) => {
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

export const fetchShips = async (renderer3d, bus, overlay) => {
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

export const setupFlightControls = (renderer3d, bus, overlay, bubbleOverlay) => {
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
    if (BUBBLE_LABELS_ENABLED) {
      bubbleOverlay?.setFlightsVisible?.(false);
    }
    toggle.addEventListener("click", () => {
      const next = toggle.getAttribute("aria-pressed") !== "true";
      toggle.setAttribute("aria-pressed", next.toString());
      overlay?.setVisible(next);
      if (BUBBLE_LABELS_ENABLED) {
        bubbleOverlay?.setFlightsVisible?.(next);
      }
      if (next) {
        fetchFlights(renderer3d, bus, overlay);
      }
    });
  }
};

export const setupSatelliteControls = (renderer3d, bus, overlay, bubbleOverlay) => {
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
    if (BUBBLE_LABELS_ENABLED) {
      bubbleOverlay?.setSatellitesVisible?.(false);
    }
    toggle.addEventListener("click", () => {
      const next = toggle.getAttribute("aria-pressed") !== "true";
      toggle.setAttribute("aria-pressed", next.toString());
      overlay?.setVisible(next);
      if (BUBBLE_LABELS_ENABLED) {
        bubbleOverlay?.setSatellitesVisible?.(next);
      }
      if (next) {
        fetchSatellites(renderer3d, bus, overlay);
      }
    });
  }
};

export const setupShipControls = (renderer3d, bus, overlay, bubbleOverlay) => {
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
    if (BUBBLE_LABELS_ENABLED) {
      bubbleOverlay?.setShipsVisible?.(false);
    }
    toggle.addEventListener("click", () => {
      const next = toggle.getAttribute("aria-pressed") !== "true";
      toggle.setAttribute("aria-pressed", next.toString());
      overlay?.setVisible(next);
      if (BUBBLE_LABELS_ENABLED) {
        bubbleOverlay?.setShipsVisible?.(next);
      }
      if (next) {
        fetchShips(renderer3d, bus, overlay);
      }
    });
  }
};

const resolveMediaUrl = (rawUrl, kind) => {
  if (!rawUrl) return rawUrl;
  const lowered = rawUrl.toLowerCase();
  if (lowered.startsWith("data:") || lowered.startsWith("blob:")) {
    return rawUrl;
  }
  let parsed;
  try {
    parsed = new URL(rawUrl, window.location.href);
  } catch (error) {
    return rawUrl;
  }
  if (parsed.protocol === "rtsp:") {
    return `/ui/rtsp-proxy?url=${encodeURIComponent(parsed.toString())}`;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    return rawUrl;
  }
  if (parsed.origin === window.location.origin) {
    return parsed.toString();
  }
  return `/ui/media-proxy?url=${encodeURIComponent(parsed.toString())}`;
};

const inferMediaKind = (rawUrl, selected) => {
  if (!rawUrl) return selected || "mjpg";
  const lowered = rawUrl.toLowerCase();
  if (lowered.startsWith("rtsp://")) return "rtsp";
  const clean = lowered.split("?")[0].split("#")[0];
  if (clean.endsWith(".m3u8") || clean.endsWith(".mpd")) return "video";
  if (
    clean.endsWith(".mp4") ||
    clean.endsWith(".m4v") ||
    clean.endsWith(".webm") ||
    clean.endsWith(".mov")
  ) {
    return "video";
  }
  if (clean.endsWith(".gif")) return "gif";
  if (
    clean.endsWith(".jpg") ||
    clean.endsWith(".jpeg") ||
    clean.endsWith(".png") ||
    clean.endsWith(".webp")
  ) {
    return "image";
  }
  if (clean.endsWith(".mjpg") || clean.endsWith(".mjpeg")) return "mjpg";
  return selected || "mjpg";
};

export const setupMediaOverlayControls = (renderer3d, overlay) => {
  const panel = document.getElementById("media-overlay-panel");
  if (!panel || !overlay) return;
  const toggle = panel.querySelector("[data-media-overlay-toggle]");
  const loadButton = panel.querySelector("[data-media-overlay-load]");
  const clearButton = panel.querySelector("[data-media-overlay-clear]");
  const playButton = panel.querySelector("[data-media-overlay-play]");
  const pauseButton = panel.querySelector("[data-media-overlay-pause]");
  const stopButton = panel.querySelector("[data-media-overlay-stop]");
  const muteButton = panel.querySelector("[data-media-overlay-mute]");
  const typeSelect = document.getElementById("media-overlay-type");
  const urlInput = document.getElementById("media-overlay-url");
  const latInput = document.getElementById("media-overlay-lat");
  const lonInput = document.getElementById("media-overlay-lon");
  const widthInput = document.getElementById("media-overlay-width");
  const heightInput = document.getElementById("media-overlay-height");
  const rotationInput = document.getElementById("media-overlay-rotation");
  const altitudeInput = document.getElementById("media-overlay-altitude");
  const scaleInput = document.getElementById("media-overlay-scale");

  const applyTransform = () => {
    const lat = clampLat(parseNumber(latInput?.value, overlay.lat));
    const lon = wrapLon(parseNumber(lonInput?.value, overlay.lon));
    const widthDeg = Math.max(1, Math.abs(parseNumber(widthInput?.value, overlay.widthDeg)));
    const heightDeg = Math.max(
      1,
      Math.abs(parseNumber(heightInput?.value, overlay.heightDeg)),
    );
    const rotationDeg = parseNumber(rotationInput?.value, overlay.rotationDeg);
    const altitude = Math.max(0, parseNumber(altitudeInput?.value, overlay.altitude));
    const scale = Math.max(0.1, parseNumber(scaleInput?.value, overlay.scale));
    overlay.setTransform({
      lat,
      lon,
      widthDeg,
      heightDeg,
      rotationDeg,
      altitude,
      scale,
    });
  };

  const applySource = () => {
    let kind = typeSelect?.value || "mjpg";
    const url = (urlInput?.value || "").trim();
    const inferred = inferMediaKind(url, kind);
    if (inferred !== kind) {
      kind = inferred;
      if (typeSelect) typeSelect.value = inferred;
    }
    const resolvedUrl = resolveMediaUrl(url, kind);
    overlay.setSource(kind, resolvedUrl);
    applyTransform();
    syncTransportButtons();
  };

  const syncTransportButtons = () => {
    const isVideo = overlay.kind === "video";
    const active = overlay.enabled && isVideo && overlay.video;
    [playButton, pauseButton, stopButton, muteButton].forEach((button) => {
      if (!button) return;
      button.disabled = !active;
    });
  };

  const setPlayback = (state) => {
    overlay.setPlayback(state);
    if (playButton) {
      playButton.setAttribute("aria-pressed", state === "playing" ? "true" : "false");
    }
    if (pauseButton) {
      pauseButton.setAttribute("aria-pressed", state === "paused" ? "true" : "false");
    }
    if (stopButton) {
      stopButton.setAttribute("aria-pressed", state === "stopped" ? "true" : "false");
    }
    syncTransportButtons();
  };

  const setMuted = (muted) => {
    overlay.setAudioMuted(muted);
    if (muteButton) {
      muteButton.setAttribute("aria-pressed", muted ? "true" : "false");
    }
    syncTransportButtons();
  };

  const setEnabled = (enabled) => {
    if (toggle) toggle.setAttribute("aria-pressed", enabled ? "true" : "false");
    overlay.setEnabled(enabled);
    syncTransportButtons();
  };

  setEnabled(false);
  applyTransform();
  setMuted(true);
  setPlayback("playing");

  toggle?.addEventListener("click", () => {
    const next = toggle.getAttribute("aria-pressed") !== "true";
    setEnabled(next);
    if (next && urlInput?.value) {
      applySource();
    }
  });

  loadButton?.addEventListener("click", () => {
    applySource();
    setEnabled(true);
    setPlayback("playing");
  });

  clearButton?.addEventListener("click", () => {
    overlay.clear();
    if (urlInput) urlInput.value = "";
    setEnabled(false);
    setMuted(true);
    setPlayback("stopped");
  });

  playButton?.addEventListener("click", () => {
    setPlayback("playing");
  });

  pauseButton?.addEventListener("click", () => {
    setPlayback("paused");
  });

  stopButton?.addEventListener("click", () => {
    setPlayback("stopped");
  });

  muteButton?.addEventListener("click", () => {
    const next = muteButton.getAttribute("aria-pressed") !== "true";
    setMuted(next);
  });

  [latInput, lonInput, widthInput, heightInput, rotationInput, altitudeInput, scaleInput].forEach(
    (input) => {
      input?.addEventListener("input", applyTransform);
    },
  );
  typeSelect?.addEventListener("change", () => {
    if (overlay.enabled) applySource();
    syncTransportButtons();
  });
  urlInput?.addEventListener("change", () => {
    if (overlay.enabled) applySource();
    syncTransportButtons();
  });
};

export const setupTileProviders = (renderer3d) => {
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

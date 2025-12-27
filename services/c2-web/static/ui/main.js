import { ecsRuntime, ECS_KIND, parseEntityId } from "/static/ui/ecs.js";
import { EntityStore, syncEntities } from "/static/ui/store.js";
import {
  BubbleOverlay,
  BubblePopup,
  FlightOverlay,
  SatelliteOverlay,
  ShipOverlay,
} from "/static/ui/overlays.js";
import {
  formatFlightDetails,
  formatSatelliteDetails,
  formatShipDetails,
} from "/static/ui/entity-utils.js";
import { EventBus } from "/static/ui/bus.js";
import { BoardView } from "/static/ui/board.js";
import { Renderer3D, MediaOverlay } from "/static/ui/renderer.js";
import { updateStatus, startSse, startWs, fetchEntities } from "/static/ui/net.js";
import {
  setupTileProviders,
  setupGlobeControls,
  setupSeaControls,
  setupWeatherControls,
  setupFlightControls,
  setupSatelliteControls,
  setupShipControls,
  setupMediaOverlayControls,
  setupLayerToggles,
  fetchFlights,
  fetchSatellites,
  fetchShips,
} from "/static/ui/controls.js";
import {
  setupDockControls,
  setupDockDrag,
  setupWindowMenuActions,
  setDockState,
  allDocks,
} from "/static/ui/docks.js";
import {
  FLIGHT_CONFIG,
  SATELLITE_CONFIG,
  SHIP_CONFIG,
  LABEL_LOD_MAX,
} from "/static/ui/config.js";
import { els } from "/static/ui/dom.js";

export const boot = async () => {
  try {
    await ecsRuntime.init();
  } catch (error) {
    console.warn("Proceeding without ECS runtime.", error);
  }
  const bus = new EventBus();
  const store = new EntityStore();
  const board = new BoardView(els.board, els.map2d);
  board.resize();

  const renderer3d = new Renderer3D(els.map3d, els.map2d);
  renderer3d.init();
  ecsRuntime.setGlobeRadius(renderer3d.globeRadius);

  const popup = new BubblePopup((action, entityId) => {
    if (action === "focus") {
      const entity = parseEntityId(entityId);
      if (entity === null) return;
      const geo = store.getComponent(entity, "Geo");
      if (geo) renderer3d.focusOnGeo(geo);
    } else {
      console.info("bubble action", { action, entityId });
    }
  }, () => {
    renderer3d.setSelectedEntity(null);
  });
  const bubbleOverlay = new BubbleOverlay(renderer3d, els.board, popup);
  bubbleOverlay.onSelect = (entityId) => {
    renderer3d.setSelectedEntity(entityId);
  };
  bubbleOverlay.resize(window.innerWidth, window.innerHeight);
  bubbleOverlay.setLodEnabled(false);
  const flightOverlay = new FlightOverlay(renderer3d, store);
  const satelliteOverlay = new SatelliteOverlay(renderer3d, store);
  const shipOverlay = new ShipOverlay(renderer3d, store);
  const mediaOverlay = new MediaOverlay(renderer3d);

  const buildPopupDetails = (entity) => {
    const meta = store.getComponent(entity, "Meta");
    if (!meta) return { label: "Entity", details: "" };
    if (meta.kind === "flight") {
      const flight = store.getComponent(entity, "Flight");
      return {
        label: store.getComponent(entity, "Pin")?.label || "Flight",
        details: formatFlightDetails(flight),
      };
    }
    if (meta.kind === "satellite") {
      const satellite = store.getComponent(entity, "Satellite");
      return {
        label: store.getComponent(entity, "Pin")?.label || "Satellite",
        details: formatSatelliteDetails(satellite),
      };
    }
    if (meta.kind === "ship") {
      const ship = store.getComponent(entity, "Ship");
      return {
        label: store.getComponent(entity, "Pin")?.label || "Ship",
        details: formatShipDetails(ship),
      };
    }
    const label =
      store.getComponent(entity, "Pin")?.label ||
      meta?.data?.name ||
      meta?.data?.summary ||
      meta.kind ||
      "Entity";
    return { label, details: "" };
  };

  const attachParticlePicker = () => {
    const canvas = renderer3d.canvas;
    if (!canvas) return;
    let pointerDown = null;
    const onPointerDown = (event) => {
      pointerDown = { x: event.clientX, y: event.clientY };
    };
    const onPointerUp = (event) => {
      if (!pointerDown) return;
      const dx = event.clientX - pointerDown.x;
      const dy = event.clientY - pointerDown.y;
      pointerDown = null;
      if (Math.hypot(dx, dy) > 6) return;
      const entity = renderer3d.pickEntity(event.clientX, event.clientY);
      if (!entity) {
        popup.close(true);
        renderer3d.setSelectedEntity(null);
        return;
      }
      const { label, details } = buildPopupDetails(entity);
      popup.openFor(entity, label, details);
      renderer3d.setSelectedEntity(entity);
    };
    canvas.addEventListener("pointerdown", onPointerDown);
    canvas.addEventListener("pointerup", onPointerUp);
  };

  bus.on("entities:update", (payload) => {
    syncEntities(payload, store);
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
  setupSeaControls(renderer3d);
  setupWeatherControls(renderer3d);
  setupFlightControls(renderer3d, bus, flightOverlay, bubbleOverlay);
  setupSatelliteControls(renderer3d, bus, satelliteOverlay, bubbleOverlay);
  setupShipControls(renderer3d, bus, shipOverlay, bubbleOverlay);
  setupMediaOverlayControls(renderer3d, mediaOverlay);

  let labelsEnabled = false;
  const updateLabelLod = (entityCount) => {
    const next = entityCount > 0 && entityCount <= LABEL_LOD_MAX;
    if (next === labelsEnabled) return;
    labelsEnabled = next;
    bubbleOverlay.setLodEnabled(labelsEnabled);
  };

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

      ecsRuntime.tick();
      const ecsFrame = ecsRuntime.refreshRenderCache();
      ecsRuntime.refreshKindCache([
        ECS_KIND.asset,
        ECS_KIND.unit,
        ECS_KIND.mission,
        ECS_KIND.incident,
        ECS_KIND.flight,
        ECS_KIND.satellite,
        ECS_KIND.ship,
      ]);
      board.drawGrid();

      const entityCount = ecsFrame?.ids?.length || 0;
      renderer3d.setParticles(ecsFrame);
      updateLabelLod(entityCount);
      flightOverlay.sync();
      satelliteOverlay.sync();
      shipOverlay.sync();
      mediaOverlay.update(now);
      renderer3d.render(delta, () => {
        if (!labelsEnabled) return;
        const kindCache = ecsRuntime.kindCache;
        const assetIds = kindCache.get(ECS_KIND.asset) || [];
        const unitIds = kindCache.get(ECS_KIND.unit) || [];
        const missionIds = kindCache.get(ECS_KIND.mission) || [];
        const incidentIds = kindCache.get(ECS_KIND.incident) || [];
        const flightIds = kindCache.get(ECS_KIND.flight) || [];
        const satelliteIds = kindCache.get(ECS_KIND.satellite) || [];
        const shipIds = kindCache.get(ECS_KIND.ship) || [];
        const markerLists = [assetIds, unitIds, missionIds, incidentIds];
        if (flightOverlay.visible) markerLists.push(flightIds);
        if (satelliteOverlay.visible) markerLists.push(satelliteIds);
        if (shipOverlay.visible) markerLists.push(shipIds);
        bubbleOverlay.syncPins(markerLists, store);
        bubbleOverlay.syncFlights(flightOverlay.visible ? flightIds : [], store);
        bubbleOverlay.syncSatellites(satelliteOverlay.visible ? satelliteIds : [], store);
        bubbleOverlay.syncShips(shipOverlay.visible ? shipIds : [], store);
        bubbleOverlay.syncEdges(markerLists, store);
      });

      if (els.runtimeStats) {
        els.runtimeStats.textContent = `Entities: ${entityCount} \u00b7 FPS: ${fps}`;
      }
      if (els.cameraStats) {
        if (renderer3d.mode === "iso" && renderer3d.camera?.isOrthographicCamera) {
          els.cameraStats.textContent = `View: Iso \u00b7 Zoom: ${renderer3d.camera.zoom.toFixed(
            2,
          )}`;
        } else if (renderer3d.camera) {
          const distance = renderer3d.camera.position.length();
          els.cameraStats.textContent = `View: Globe \u00b7 Dist: ${Math.round(distance)}`;
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
    bubbleOverlay.resize(renderer3d.size.width, renderer3d.size.height);
  };

  window.addEventListener("resize", resize);
  resize();
  attachParticlePicker();

  updateStatus();
  fetchEntities(bus);
  startSse(bus);
  startWs(bus);
  setInterval(updateStatus, 15000);
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
  setupDockControls();
  setupDockDrag();
  setupWindowMenuActions();
  allDocks().forEach((dock) => {
    setDockState(dock, dock.dataset.state || "open");
  });
  setupLayerToggles(renderer3d, bubbleOverlay);

  requestAnimationFrame(renderLoop);
};

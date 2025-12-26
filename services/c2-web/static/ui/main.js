import { ecsRuntime, ECS_KIND, parseEntityId } from "/static/ui/ecs.js";
import { EntityStore, syncEntities } from "/static/ui/store.js";
import {
  BubbleOverlay,
  BubblePopup,
  FlightOverlay,
  SatelliteOverlay,
  ShipOverlay,
} from "/static/ui/overlays.js";
import { shipBaseAltitude, altitudeForShip } from "/static/ui/entity-utils.js";
import { forEachEntity } from "/static/ui/utils.js";
import { EventBus } from "/static/ui/bus.js";
import { BoardView } from "/static/ui/board.js";
import { Renderer3D, MediaOverlay } from "/static/ui/renderer.js";
import { updateStatus, startSse, startWs, fetchEntities } from "/static/ui/net.js";
import {
  setupTileProviders,
  setupGlobeControls,
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
import { FLIGHT_CONFIG, SATELLITE_CONFIG, SHIP_CONFIG } from "/static/ui/config.js";
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

  const bubbleOverlay = new BubbleOverlay(
    renderer3d,
    els.board,
    new BubblePopup((action, entityId) => {
      if (action === "focus") {
        const entity = parseEntityId(entityId);
        if (entity === null) return;
        const geo = store.getComponent(entity, "Geo");
        if (geo) renderer3d.focusOnGeo(geo);
      } else {
        console.info("bubble action", { action, entityId });
      }
    }, () => bubbleOverlay?.clearSelection?.()),
  );
  bubbleOverlay.resize(window.innerWidth, window.innerHeight);
  const flightOverlay = new FlightOverlay(renderer3d, store);
  const satelliteOverlay = new SatelliteOverlay(renderer3d, store);
  const shipOverlay = new ShipOverlay(renderer3d, store);
  const mediaOverlay = new MediaOverlay(renderer3d);

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
  setupWeatherControls(renderer3d);
  setupFlightControls(renderer3d, bus, flightOverlay, bubbleOverlay);
  setupSatelliteControls(renderer3d, bus, satelliteOverlay, bubbleOverlay);
  setupShipControls(renderer3d, bus, shipOverlay, bubbleOverlay);
  setupMediaOverlayControls(renderer3d, mediaOverlay);

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
      const kindCache = ecsRuntime.refreshKindCache([
        ECS_KIND.asset,
        ECS_KIND.unit,
        ECS_KIND.mission,
        ECS_KIND.incident,
        ECS_KIND.flight,
        ECS_KIND.satellite,
        ECS_KIND.ship,
      ]);
      board.drawGrid();

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
      const shipAltitude = shipBaseAltitude(renderer3d) + altitudeForShip();
      const useEcsPositions = ecsFrame && renderer3d.mode === "globe";
      let entityCount = 0;
      const points = [];
      forEachEntity(markerLists, (entity) => {
        const geo = store.getComponent(entity, "Geo");
        const renderable = store.getComponent(entity, "Renderable") || {};
        const meta = store.getComponent(entity, "Meta");
        const altitude =
          meta?.kind === "ship" ? shipAltitude : renderer3d.markerAltitude;
        let pos = null;
        if (useEcsPositions) {
          pos = ecsRuntime.positionForId(entity, altitude, renderer3d.globeRadius);
        }
        if (!pos) {
          pos = renderer3d.positionForGeo(geo, altitude);
        }
        points.push({ ...pos, color: renderable.color });
        entityCount += 1;
      });
      renderer3d.setInstances(points);
      flightOverlay.sync();
      satelliteOverlay.sync();
      shipOverlay.sync();
      mediaOverlay.update(now);
      renderer3d.render(delta, () => {
        const pinLists = [assetIds, unitIds, missionIds, incidentIds];
        const flightEntities = flightOverlay.visible ? flightIds : [];
        const satelliteEntities = satelliteOverlay.visible ? satelliteIds : [];
        const shipEntities = shipOverlay.visible ? shipIds : [];
        bubbleOverlay.syncPins(pinLists, store);
        bubbleOverlay.syncFlights(flightEntities, store);
        bubbleOverlay.syncSatellites(satelliteEntities, store);
        bubbleOverlay.syncShips(shipEntities, store);
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
  setupLayerToggles(bubbleOverlay);

  requestAnimationFrame(renderLoop);
};

# Frontend Systems Plan

This document defines the frontend design patterns for a layered C2 console with 2D/3D rendering and large-scale entity updates.

## Goals

- Infinite board layout with layered rendering (3D + 2D + DOM overlays).
- High-entity throughput using an ECS architecture and instanced rendering.
- Real-time updates through SSE/WS, with deterministic fallbacks for offline/edge modes.
- UI composition similar to Excalidraw (canvas-first) with mission overlays and sliding panels.

## Layer Stack (Rendering)

- Layer 0: 3D map canvas (WebGL) for terrain, volumetrics, and spatial anchors.
- Layer 1: 2D canvas for grids, contours, and fast vector overlays.
- Layer 2: DOM overlay for labels, pins, and interactive UI widgets.
- HUD layer: persistent status, timeline, and alert rails.

Each layer receives a unified camera transform so 2D/3D and DOM pins align.

Current globe rendering uses locally stored 8k Earth textures from Solar System Scope
(day/night/clouds/normal/specular) for offline-capable visualization. Swap with an internal
tile server or terrain service when available.

## Infinite Board

- The board uses a world coordinate system (meters or abstract units).
- View state is controlled by pan + zoom (2D) and camera position/orientation (3D).
- Pinning anchors map world coordinates to screen positions.

## ECS Architecture

- Entities are numeric IDs.
- Components are typed data blobs (Transform, Renderable, Status, Assignment, etc.).
- Systems are pure update loops (Render2D, Render3D, StatusColoring, PinSync).

Key patterns:
- `World.query([Transform, Renderable])` for fast iteration.
- Instanced meshes in WebGL for thousands of markers.
- 2D canvas batch rendering for labels and overlays.

## Realtime Data Flow

- SSE/WS streams feed an EventBus.
- EventBus normalizes updates into ECS component changes.
- UI partial updates continue via existing Tera partials for non-map panels.

## Panel + Overlay Model

- Left dock: tools, layers, and mission controls.
- Right dock: inspector, entity details, and alerts.
- Bottom bar: timeline, playback, and filters.
- Overlay stack: popups and pin-bound inspectors.

## Performance Guardrails

- RequestAnimationFrame render loop with capped updates.
- Debounced UI updates for panels.
- Entity culling by camera bounds.
- Progressive detail (icons first, labels later).

## Security/Offline Considerations

- All assets are local, no CDN runtime dependencies.
- Updates are signed by API sessions (future policy engine integration).
- Offline mode replays cached events into ECS once reconnected.

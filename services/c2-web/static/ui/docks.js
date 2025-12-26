import { els } from "/static/ui/dom.js";

const DOCK_VISIBLE_CLASSES = ["opacity-100", "pointer-events-auto", "scale-100"];
const DOCK_HIDDEN_CLASSES = ["opacity-0", "pointer-events-none", "scale-95"];
const DOCK_EXPANDED_CLASSES = ["h-[70vh]", "min-h-[320px]", "resize"];
const DOCK_MINIMIZED_CLASSES = ["h-auto", "min-h-0", "resize-none"];
const DOCK_CENTER_CLASSES = [
  "left-1/2",
  "top-1/2",
  "-translate-x-1/2",
  "-translate-y-1/2",
];
const dockStates = ["open", "minimized", "closed"];
let dockZ = 40;

const normalizeDockState = (state) =>
  dockStates.includes(state) ? state : "open";

const bringDockToFront = (dock) => {
  dockZ += 1;
  dock.style.zIndex = dockZ.toString();
};

const updateDockControls = (dock) => {
  if (!dock) return;
  const state = normalizeDockState(dock.dataset.state);
  const minimize = dock.querySelector('[data-dock-action="minimize"]');
  if (!minimize) return;
  if (state === "minimized") {
    minimize.textContent = "+";
    minimize.setAttribute("aria-label", "Restore window");
  } else {
    minimize.textContent = "\u2014";
    minimize.setAttribute("aria-label", "Minimize window");
  }
};

const storeDockSize = (dock) => {
  if (!dock) return;
  if (dock.style.width) {
    dock.dataset.savedWidth = dock.style.width;
  }
  if (dock.style.height) {
    dock.dataset.savedHeight = dock.style.height;
  }
};

const restoreDockSize = (dock) => {
  if (!dock) return;
  if (dock.dataset.savedWidth) {
    dock.style.width = dock.dataset.savedWidth;
  } else {
    dock.style.removeProperty("width");
  }
  if (dock.dataset.savedHeight) {
    dock.style.height = dock.dataset.savedHeight;
  } else {
    dock.style.removeProperty("height");
  }
  dock.style.removeProperty("minHeight");
};

const clearDockHeight = (dock) => {
  if (!dock) return;
  dock.style.removeProperty("height");
  dock.style.removeProperty("minHeight");
};

const applyDockCentered = (dock, centered) => {
  if (!dock) return;
  if (centered) {
    dock.classList.add(...DOCK_CENTER_CLASSES);
  } else {
    dock.classList.remove(...DOCK_CENTER_CLASSES);
  }
};

const applyDockStateClasses = (dock, state) => {
  if (!dock) return;
  dock.classList.remove(
    ...DOCK_VISIBLE_CLASSES,
    ...DOCK_HIDDEN_CLASSES,
    ...DOCK_EXPANDED_CLASSES,
    ...DOCK_MINIMIZED_CLASSES,
  );
  if (state === "closed") {
    dock.classList.add(...DOCK_HIDDEN_CLASSES);
  } else {
    dock.classList.add(...DOCK_VISIBLE_CLASSES);
  }
  if (state === "minimized") {
    dock.classList.add(...DOCK_MINIMIZED_CLASSES);
  } else {
    dock.classList.add(...DOCK_EXPANDED_CLASSES);
  }
  const content = dock.querySelector(".dock-content");
  if (content) {
    content.classList.toggle("hidden", state === "minimized");
  }
};

const positionDockCenter = (dock) => {
  const parent = dock.offsetParent || document.body;
  const parentRect = parent.getBoundingClientRect();
  const width = dock.offsetWidth || 320;
  const height = dock.offsetHeight || 420;
  const left = Math.max(12, (parentRect.width - width) / 2);
  const top = Math.max(12, (parentRect.height - height) / 2);
  dock.style.left = `${left}px`;
  dock.style.top = `${top}px`;
  dock.dataset.positioned = "true";
  applyDockCentered(dock, false);
};

const releaseDockFocus = (dock) => {
  const active = document.activeElement;
  if (!active || !dock.contains(active)) return;
  if (active.blur) active.blur();
  const fallback = document.querySelector("[data-focus-fallback]");
  if (fallback && fallback.focus) {
    fallback.focus({ preventScroll: true });
  }
};

export const setDockState = (dock, state) => {
  if (!dock) return;
  const next = normalizeDockState(state);
  const current = normalizeDockState(dock.dataset.state);
  if (next === "closed") {
    releaseDockFocus(dock);
  }
  if (next === "minimized" && current !== "minimized") {
    storeDockSize(dock);
    clearDockHeight(dock);
  }
  if (next === "open" && current === "minimized") {
    restoreDockSize(dock);
  }
  dock.dataset.state = next;
  dock.setAttribute("aria-hidden", next === "closed" ? "true" : "false");
  applyDockStateClasses(dock, next);
  if (next === "closed") {
    dock.setAttribute("inert", "");
  } else {
    dock.removeAttribute("inert");
  }
  if (next === "open") {
    if (dock.dataset.positioned !== "true") {
      applyDockCentered(dock, true);
      positionDockCenter(dock);
    }
    bringDockToFront(dock);
  }
  updateDockControls(dock);
  updateWindowMenuState();
};

const toggleDockState = (dock) => {
  if (!dock) return;
  const state = normalizeDockState(dock.dataset.state);
  const next = state === "open" ? "minimized" : "open";
  setDockState(dock, next);
};

const updateWindowMenuState = () => {
  document.querySelectorAll("[data-window-state]").forEach((node) => {
    const id = node.dataset.windowState;
    const dock = document.getElementById(id);
    if (!dock) return;
    const state = normalizeDockState(dock.dataset.state);
    node.dataset.state = state;
    node.textContent =
      state === "open" ? "Open" : state === "minimized" ? "Minimized" : "Closed";
  });
};

const applyDockAction = (dock, action) => {
  if (!dock) return;
  if (action === "minimize") {
    const current = normalizeDockState(dock.dataset.state);
    setDockState(dock, current === "minimized" ? "open" : "minimized");
    return;
  }
  if (action === "close") {
    setDockState(dock, "closed");
    return;
  }
  if (action === "open") {
    setDockState(dock, "open");
  }
};

export const allDocks = () => [els.dockLeft, els.dockRight].filter(Boolean);

export const setupDockControls = () => {
  document.querySelectorAll("[data-dock-action]").forEach((button) => {
    const action = button.dataset.dockAction;
    const dock = button.closest(".dock");
    if (!action || !dock) return;
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      applyDockAction(dock, action);
    });
  });
};

export const setupDockDrag = () => {
  document.querySelectorAll(".dock").forEach((dock) => {
    dock.addEventListener("pointerdown", () => bringDockToFront(dock));
  });
  document.querySelectorAll("[data-dock-drag-handle]").forEach((handle) => {
    handle.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      const dock = handle.closest(".dock");
      if (!dock || normalizeDockState(dock.dataset.state) === "closed") return;
      event.preventDefault();
      bringDockToFront(dock);
      const parent = dock.offsetParent || document.body;
      const parentRect = parent.getBoundingClientRect();
      const rect = dock.getBoundingClientRect();
      const offsetX = event.clientX - rect.left;
      const offsetY = event.clientY - rect.top;
      dock.style.left = `${rect.left - parentRect.left}px`;
      dock.style.top = `${rect.top - parentRect.top}px`;
      dock.dataset.positioned = "true";
      dock.classList.add("dragging");
      applyDockCentered(dock, false);
      const header = dock.querySelector(".dock-header");
      header?.classList.add("cursor-grabbing");

      const onMove = (moveEvent) => {
        const width = dock.offsetWidth;
        const height = dock.offsetHeight;
        const maxLeft = Math.max(12, parentRect.width - width - 12);
        const maxTop = Math.max(12, parentRect.height - height - 12);
        const nextLeft = Math.min(
          maxLeft,
          Math.max(12, moveEvent.clientX - parentRect.left - offsetX),
        );
        const nextTop = Math.min(
          maxTop,
          Math.max(12, moveEvent.clientY - parentRect.top - offsetY),
        );
        dock.style.left = `${nextLeft}px`;
        dock.style.top = `${nextTop}px`;
      };

      const onUp = () => {
        dock.classList.remove("dragging");
        const header = dock.querySelector(".dock-header");
        header?.classList.remove("cursor-grabbing");
        window.removeEventListener("pointermove", onMove);
      };

      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp, { once: true });
    });
  });
};

export const setupWindowMenuActions = () => {
  document.querySelectorAll("[data-window-action]").forEach((button) => {
    const action = button.dataset.windowAction;
    const target = button.dataset.windowId;
    button.addEventListener("click", () => {
      if (action === "toggle" && target) {
        const dock = document.getElementById(target);
        const state = normalizeDockState(dock?.dataset?.state);
        setDockState(dock, state === "open" ? "minimized" : "open");
        return;
      }
      if (action === "open-all") {
        allDocks().forEach((dock) => setDockState(dock, "open"));
        return;
      }
      if (action === "minimize-all") {
        allDocks().forEach((dock) => setDockState(dock, "minimized"));
        return;
      }
      if (action === "close-all") {
        allDocks().forEach((dock) => setDockState(dock, "closed"));
        return;
      }
    });
  });
};

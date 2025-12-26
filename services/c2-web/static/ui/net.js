import { els } from "/static/ui/dom.js";

const STATUS_DOT_IDLE_CLASS = "bg-orange-500 shadow-[0_0_12px_rgba(214,90,49,0.2)]";
const STATUS_DOT_STATE_CLASSES = {
  ok: "bg-emerald-500 shadow-[0_0_12px_rgba(22,163,74,0.35)]",
  warn: "bg-amber-500 shadow-[0_0_12px_rgba(245,158,11,0.35)]",
  error: "bg-red-600 shadow-[0_0_12px_rgba(220,38,38,0.35)]",
};
const STATUS_DOT_CLASS_LIST = Array.from(
  new Set(
    [STATUS_DOT_IDLE_CLASS, ...Object.values(STATUS_DOT_STATE_CLASSES)]
      .flatMap((value) => value.split(/\s+/))
      .filter(Boolean),
  ),
);

const setDot = (state) => {
  if (!els.apiDot) return;
  if (!state || !STATUS_DOT_STATE_CLASSES[state]) return;
  if (STATUS_DOT_CLASS_LIST.length) {
    els.apiDot.classList.remove(...STATUS_DOT_CLASS_LIST);
  }
  els.apiDot.classList.add(...STATUS_DOT_STATE_CLASSES[state].split(/\s+/));
};

const swapHtml = (targetId, html) => {
  const el = document.getElementById(targetId);
  if (!el) return;
  el.innerHTML = html;
};

const applyPartialBatch = (payload) => {
  if (!payload || !Array.isArray(payload.fragments)) return;
  payload.fragments.forEach((fragment) => {
    if (!fragment || !fragment.target) return;
    swapHtml(fragment.target, fragment.html || "");
  });
};

export const updateStatus = async () => {
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

export const startSse = (bus) => {
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

export const startWs = (bus) => {
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

export const fetchEntities = async (bus) => {
  try {
    const response = await fetch("/ui/entities", { cache: "no-store" });
    if (!response.ok) return;
    const payload = await response.json();
    bus.emit("entities:update", payload);
  } catch {
    // ignore entity fetch errors
  }
};

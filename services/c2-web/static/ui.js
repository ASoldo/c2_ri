(() => {
  const apiStatusEl = document.getElementById("api-status");
  const apiDotEl = document.getElementById("api-dot");
  const streamStatusEl = document.getElementById("stream-status");
  const wsStatusEl = document.getElementById("ws-status");
  const partialEls = Array.from(document.querySelectorAll("[data-partial]"));

  const setDot = (state) => {
    if (!apiDotEl) return;
    apiDotEl.classList.remove("ok", "warn", "error");
    if (state) apiDotEl.classList.add(state);
  };

  const swapHtml = (targetId, html) => {
    const el = document.getElementById(targetId);
    if (!el) return;
    el.innerHTML = html;
  };

  const refreshPartials = async () => {
    await Promise.all(
      partialEls.map(async (el) => {
        const url = el.dataset.partial;
        if (!url) return;
        try {
          const response = await fetch(url, { cache: "no-store" });
          if (!response.ok) return;
          const html = await response.text();
          el.innerHTML = html;
        } catch (err) {
          // ignore partial refresh errors
        }
      }),
    );
  };

  const updateStatus = async () => {
    if (!apiStatusEl) return;
    try {
      const response = await fetch("/ui/status", { cache: "no-store" });
      if (!response.ok) throw new Error("status fetch failed");
      const data = await response.json();
      apiStatusEl.textContent = `API: ${data.service} (${data.environment})`;
      setDot("ok");
    } catch (err) {
      apiStatusEl.textContent = "API: unavailable";
      setDot("warn");
    }
  };

  const applyPartialBatch = (payload) => {
    if (!payload || !Array.isArray(payload.fragments)) return;
    payload.fragments.forEach((fragment) => {
      if (!fragment || !fragment.target) return;
      swapHtml(fragment.target, fragment.html || "");
    });
  };

  const startSse = () => {
    if (!streamStatusEl || !window.EventSource) return;
    const source = new EventSource("/ui/stream/sse");
    streamStatusEl.textContent = "SSE: connecting";
    source.addEventListener("partials", (event) => {
      streamStatusEl.textContent = "SSE: live";
      const payload = JSON.parse(event.data || "{}");
      applyPartialBatch(payload);
    });
    source.addEventListener("error", () => {
      streamStatusEl.textContent = "SSE: reconnecting";
    });
    source.onerror = () => {
      streamStatusEl.textContent = "SSE: reconnecting";
    };
  };

  const startWs = () => {
    if (!wsStatusEl || !window.WebSocket) return;
    const scheme = window.location.protocol === "https:" ? "wss" : "ws";
    const socket = new WebSocket(`${scheme}://${window.location.host}/ui/stream/ws`);
    wsStatusEl.textContent = "WS: connecting";
    socket.onopen = () => {
      wsStatusEl.textContent = "WS: live";
    };
    socket.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data || "{}");
        if (message.kind === "partials") {
          applyPartialBatch(message.payload);
        }
      } catch (err) {
        // ignore parse errors
      }
    };
    socket.onclose = () => {
      wsStatusEl.textContent = "WS: reconnecting";
      setTimeout(startWs, 3000);
    };
    socket.onerror = () => {
      wsStatusEl.textContent = "WS: reconnecting";
    };
  };

  document.addEventListener("DOMContentLoaded", () => {
    updateStatus();
    refreshPartials();
    startSse();
    startWs();
    setInterval(updateStatus, 15000);
    setInterval(refreshPartials, 12000);
  });
})();

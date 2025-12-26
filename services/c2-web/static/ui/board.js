export class BoardView {
  constructor(boardEl, canvas2d) {
    this.boardEl = boardEl;
    this.canvas = canvas2d;
    this.ctx = canvas2d?.getContext("2d");
    this.offset = { x: 0, y: 0 };
    this.zoom = 1.0;
    this.isPanning = false;
    this.last = { x: 0, y: 0 };
  }

  resize() {
    if (!this.canvas || !this.boardEl) return;
    const rect = this.boardEl.getBoundingClientRect();
    this.canvas.width = rect.width * devicePixelRatio;
    this.canvas.height = rect.height * devicePixelRatio;
    this.ctx?.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
  }

  bindInputs() {
    if (!this.boardEl) return;
    this.boardEl.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      this.isPanning = true;
      this.last = { x: event.clientX, y: event.clientY };
    });
    window.addEventListener("pointerup", () => {
      this.isPanning = false;
    });
    window.addEventListener("pointermove", (event) => {
      if (!this.isPanning) return;
      const dx = event.clientX - this.last.x;
      const dy = event.clientY - this.last.y;
      this.offset.x += dx;
      this.offset.y += dy;
      this.last = { x: event.clientX, y: event.clientY };
    });
    this.boardEl.addEventListener("wheel", (event) => {
      event.preventDefault();
      const delta = Math.sign(event.deltaY) * -0.1;
      this.zoom = Math.min(4, Math.max(0.4, this.zoom + delta));
    });
  }

  worldToScreen(point) {
    if (!this.boardEl) return { x: 0, y: 0 };
    const rect = this.boardEl.getBoundingClientRect();
    return {
      x: rect.width / 2 + point.x * this.zoom + this.offset.x,
      y: rect.height / 2 + point.y * this.zoom + this.offset.y,
    };
  }

  drawGrid() {
    if (!this.ctx || !this.boardEl || !this.canvas) return;
    if (this.canvas.style.display === "none") return;
    const rect = this.boardEl.getBoundingClientRect();
    this.ctx.clearRect(0, 0, rect.width, rect.height);
    const spacing = 48 * this.zoom;
    this.ctx.strokeStyle = "rgba(15, 23, 42, 0.08)";
    this.ctx.lineWidth = 1;
    for (let x = this.offset.x % spacing; x < rect.width; x += spacing) {
      this.ctx.beginPath();
      this.ctx.moveTo(x, 0);
      this.ctx.lineTo(x, rect.height);
      this.ctx.stroke();
    }
    for (let y = this.offset.y % spacing; y < rect.height; y += spacing) {
      this.ctx.beginPath();
      this.ctx.moveTo(0, y);
      this.ctx.lineTo(rect.width, y);
      this.ctx.stroke();
    }
  }
}

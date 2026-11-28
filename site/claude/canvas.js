// Pan across the hero workspace. Groups repeat horizontally and carry their own panes and wires.
const stage = document.querySelector("[data-stage]");
const app = stage.querySelector("[data-app]");
const PAD = 16;
const GAP = 24;
const LINKS = [["t1", "browser"], ["t1", "sim"], ["subscription-agent", "subscriptions"],
  ["subscription-agent", "subscription-review"], ["inventory", "inventory-tests"]];

const groups = [...stage.querySelectorAll("[data-world]")].map((el) => ({
  el,
  edges: el.querySelector("[data-edges]"),
  panes: [...el.querySelectorAll("[data-pane]")].map((pane) => ({
    el: pane, id: pane.dataset.pane, x: +pane.dataset.x, y: +pane.dataset.y,
    w: pane.offsetWidth, h: pane.offsetHeight,
  })),
}));
const panes = groups.flatMap((group) => group.panes);
const byId = Object.fromEntries(panes.map((p) => [p.id, p]));
const WORLD = {
  w: Math.max(...panes.map((p) => p.x + p.w)),
  h: Math.max(...panes.map((p) => p.y + p.h)),
};
const span = WORLD.w + 2 * PAD;
const period = span * groups.length;
const bounds = { x0: -PAD, x1: WORLD.w + PAD, y0: -PAD, y1: WORLD.h + PAD };
stage.style.aspectRatio = `${span} / ${WORLD.h + 2 * PAD}`;
for (const p of panes) place(p);

const view = { k: 1, x: 0, y: 0, panX: 0, panY: 0 };
const wrap = (x) => ((x + period / 2) % period + period) % period - period / 2;
function paintView() {
  for (const [i, group] of groups.entries()) {
    const x = view.x + wrap(i * span + view.panX) * view.k;
    const y = view.y + view.panY * view.k;
    group.el.style.transform = `translate(${x}px, ${y}px) scale(${view.k})`;
  }
}
function fit() {
  const W = app.offsetWidth, H = app.offsetHeight;
  // Keep a group offscreen when it wraps, even in a very wide, short window.
  view.k = Math.max(W / (period - span - 2 * PAD), Math.min(W / span, H / (WORLD.h + 2 * PAD)));
  view.x = (W - WORLD.w * view.k) / 2;
  view.y = (H - WORLD.h * view.k) / 2;
  paintView();
}
function pan(x, y) {
  view.panX = wrap(x);
  view.panY = Math.max(-96, Math.min(96, y));
  paintView();
}
function clamp(p) {
  p.x = Math.min(Math.max(p.x, bounds.x0), bounds.x1 - p.w);
  p.y = Math.min(Math.max(p.y, bounds.y0), bounds.y1 - p.h);
}

// --- wires: the curve from scenes.js, with a dot at each end like the app draws -------------
const NS = "http://www.w3.org/2000/svg";
const links = LINKS.filter((pair) => pair.every((id) => byId[id])).map(([a, b]) => {
  const { edges: edgesSvg } = groups.find((group) => group.panes.includes(byId[a]));
  const path = edgesSvg.appendChild(document.createElementNS(NS, "path"));
  path.setAttribute("class", "wire");
  const ends = [0, 1].map(() => {
    const c = edgesSvg.appendChild(document.createElementNS(NS, "circle"));
    c.setAttribute("class", "wire-end");
    c.setAttribute("r", "3.5");
    return c;
  });
  return { a: byId[a], b: byId[b], path, ends };
});

// The pane's drawn position, which GSAP tweens while it glides, so wires follow it.
function paintPane(p) {
  p.el.style.transform = `translate(${p.drawn.x}px, ${p.drawn.y}px)`;
}
function place(p) {
  p.drawn ??= { w: p.w, h: p.h };
  gsap.killTweensOf(p.drawn);
  p.drawn.x = p.x;
  p.drawn.y = p.y;
  paintPane(p);
}
function drawLinks() {
  for (const { a, b, path, ends } of links) {
    const { d, p1, p2 } = Scenes.wirePath(a.drawn, b.drawn);
    path.setAttribute("d", d);
    ends[0].setAttribute("cx", p1[0]); ends[0].setAttribute("cy", p1[1]);
    ends[1].setAttribute("cx", p2[0]); ends[1].setAttribute("cy", p2[1]);
  }
}

// --- making room: no two panes overlap -----------------------------------------------------------
// True when `a` and `b` (plus the gap) overlap.
const touching = (a, b) =>
  Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x) + GAP > 0 &&
  Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y) + GAP > 0;
// The free spot nearest (x, y) for pane `p`, clear of every `settled` box and inside `b`, or null. The
// candidates are (x, y) itself and positions flush against each settled box or the canvas edge.
function nearestFree(p, x, y, settled, b) {
  const free = (cx, cy) => cx >= b.x0 && cx + p.w <= b.x1 && cy >= b.y0 && cy + p.h <= b.y1 &&
    !settled.some((s) => touching(s, { x: cx, y: cy, w: p.w, h: p.h }));
  if (free(x, y)) return [x, y];
  const xs = [x, b.x0, b.x1 - p.w, ...settled.flatMap((s) => [s.x + s.w + GAP, s.x - GAP - p.w, s.x])];
  const ys = [y, b.y0, b.y1 - p.h, ...settled.flatMap((s) => [s.y + s.h + GAP, s.y - GAP - p.h, s.y])];
  let best = null, cost = Infinity;
  for (const cx of xs) for (const cy of ys) {
    const c = Math.hypot(cx - x, cy - y);
    if (c < cost && free(cx, cy)) { best = [cx, cy]; cost = c; }
  }
  return best;
}
const orders = (list) => list.length < 2 ? [list] :
  list.flatMap((p, i) => orders([...list.slice(0, i), ...list.slice(i + 1)]).map((rest) => [p, ...rest]));

// Lays the other panes in this group out around `dragged`, starting from `home`. Try every order
// within the group (at most three neighbours), so the arrangement that moves them least wins. Returns
// false, changing nothing, when no order leaves every pane clear: the dragged pane is over a spot with no
// room for the rest.
function makeRoom(panes, dragged, home) {
  let best = null;
  for (const order of orders(panes.filter((p) => p !== dragged))) {
    const settled = [dragged], spots = [];
    let cost = 0;
    for (const p of order) {
      const [hx, hy] = home.get(p);
      const spot = nearestFree(p, hx, hy, settled, bounds);
      if (!spot) { cost = Infinity; break; }
      cost += Math.hypot(spot[0] - hx, spot[1] - hy);
      if (cost >= (best?.cost ?? Infinity)) break; // already worse than the best order
      spots.push([p, spot]);
      settled.push({ x: spot[0], y: spot[1], w: p.w, h: p.h });
    }
    if (cost < (best?.cost ?? Infinity)) best = { cost, spots };
  }
  if (!best) return false;
  for (const [p, [x, y]] of best.spots) { p.x = x; p.y = y; }
  return true;
}
function glide(p) {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) { place(p); drawLinks(); return; }
  gsap.to(p.drawn, { x: p.x, y: p.y, duration: .32, ease: "power3.out", overwrite: true, onUpdate: () => { paintPane(p); drawLinks(); } });
}

// Primary drag moves a pane by its handle or pans from the canvas. The middle button pans anywhere.
let active = false;
app.addEventListener("pointerdown", (event) => {
  if (active || !event.isPrimary || (event.button !== 0 && event.button !== 1)) return;
  const head = event.button === 0 && event.target.closest(".pane-head");
  const { clientX: sx, clientY: sy, pointerId } = event;
  const k = view.k * app.getBoundingClientRect().width / app.offsetWidth;
  const move = head ? dragPane(head, sx, sy, k) : dragCanvas(sx, sy, k);
  if (event.button === 1) event.preventDefault();
  active = true;
  app.setPointerCapture(pointerId);
  const update = (e) => { if (e.pointerId === pointerId) move.update(e); };
  const end = (e) => {
    if (e.pointerId !== pointerId) return;
    move.end();
    active = false;
    app.removeEventListener("pointermove", update);
    app.removeEventListener("lostpointercapture", end);
  };
  app.addEventListener("pointermove", update);
  app.addEventListener("lostpointercapture", end);
});
app.addEventListener("mousedown", (event) => { if (event.button === 1) event.preventDefault(); });
app.addEventListener("auxclick", (event) => { if (event.button === 1) event.preventDefault(); });

function dragCanvas(sx, sy, k) {
  const { panX, panY } = view;
  app.classList.add("panning");
  return {
    update: (e) => pan(panX + (e.clientX - sx) / k, panY + (e.clientY - sy) / k),
    end: () => app.classList.remove("panning"),
  };
}

function dragPane(head, sx, sy, k) {
  const { panes } = groups.find(({ el }) => el.contains(head));
  const pane = panes.find((p) => p.el.contains(head));
  const { x: ox, y: oy } = pane.drawn;
  // Each move starts from the original layout, so neighbours return as the dragged pane moves on.
  const home = new Map(panes.map((p) => [p, [p.x, p.y]]));
  let good = home, fits = true;
  pane.el.classList.add("lifted");
  gsap.killTweensOf(pane.drawn);
  return {
    update(e) {
      pane.x = ox + (e.clientX - sx) / k;
      pane.y = oy + (e.clientY - sy) / k;
      clamp(pane);
      const previous = panes.map((p) => [p.x, p.y]);
      fits = makeRoom(panes, pane, home);
      if (fits) good = new Map(panes.map((p) => [p, [p.x, p.y]]));
      else for (const p of panes) if (p !== pane) [p.x, p.y] = good.get(p);
      place(pane);
      panes.forEach((p, i) => {
        const [x, y] = previous[i];
        if (p !== pane && (p.x !== x || p.y !== y)) glide(p);
      });
      drawLinks();
    },
    end() {
      if (!fits) { [pane.x, pane.y] = good.get(pane); glide(pane); }
      pane.el.classList.remove("lifted");
    },
  };
}

// Horizontal trackpad and Shift-scroll pan. Plain vertical scroll stays with the page.
app.addEventListener("wheel", (event) => {
  const { deltaX, deltaY, deltaMode, shiftKey, ctrlKey, metaKey } = event;
  if (active || ctrlKey || metaKey || (!shiftKey && Math.abs(deltaX) <= Math.abs(deltaY))) return;
  const dx = deltaX || deltaY;
  if (!dx) return;
  event.preventDefault();
  event.stopPropagation(); // keep diagonal trackpad gestures away from the page's Lenis handler
  const units = deltaMode === 1 ? 16 : deltaMode === 2 ? app.offsetWidth : 1;
  pan(view.panX - dx * units / view.k, view.panY);
}, { passive: false });

app.addEventListener("keydown", (event) => {
  const offset = {
    ArrowLeft: [120, 0], ArrowRight: [-120, 0], ArrowUp: [0, 48], ArrowDown: [0, -48],
  }[event.key];
  if (!offset || active || event.ctrlKey || event.metaKey || event.altKey) return;
  event.preventDefault();
  pan(view.panX + offset[0], view.panY + offset[1]);
});

new ResizeObserver(fit).observe(app);
fit();
drawLinks();

// The canopy canvas in claude.html's hero: its panes (drawn by scenes.js) that move by their headers
// and nothing else (no pan, no zoom), and wires that follow them. Uses scenes.js for wire geometry.

const stage = document.querySelector("[data-stage]");
const app = stage.querySelector("[data-app]");
const world = stage.querySelector("[data-world]");
const edgesSvg = stage.querySelector("[data-edges]");

const PAD = 16; // world units kept clear around the panes
const GAP = 24; // clear space panes keep between each other
// Who talks to whom: Claude drives the browser and the phone. Codex works alone, wired to nothing.
const LINKS = [["t1", "browser"], ["t1", "sim"]];


// --- panes ------------------------------------------------------------------------------
const panes = [...stage.querySelectorAll("[data-pane]")].map((el) => ({
  el, id: el.dataset.pane, x: +el.dataset.x, y: +el.dataset.y, w: el.offsetWidth, h: el.offsetHeight,
}));
const byId = Object.fromEntries(panes.map((p) => [p.id, p]));
// The panes' bounding box, in world units. The stage takes its shape from it, so the world always fits the width.
const WORLD = { w: Math.max(...panes.map((p) => p.x + p.w)), h: Math.max(...panes.map((p) => p.y + p.h)) };
stage.style.aspectRatio = `${WORLD.w + 2 * PAD} / ${WORLD.h + 2 * PAD}`;
for (const p of panes) place(p);

// --- view: the world is scaled to fit the frame and centred; the user cannot pan it ------------
const view = { k: 1, x: 0, y: 0 };
function fit() {
  const W = app.offsetWidth, H = app.offsetHeight;
  view.k = Math.min(W / (WORLD.w + 2 * PAD), H / (WORLD.h + 2 * PAD));
  view.x = (W - WORLD.w * view.k) / 2;
  view.y = (H - WORLD.h * view.k) / 2;
  world.style.transform = `translate(${view.x}px, ${view.y}px) scale(${view.k})`;
}
new ResizeObserver(() => { fit(); keepInside(); }).observe(app);

// The visible canvas in world units, less a 12px margin. There is no panning, so panes stay inside it.
function bounds() {
  const pad = 12 / view.k;
  return {
    x0: -view.x / view.k + pad, x1: (app.offsetWidth - view.x) / view.k - pad,
    y0: -view.y / view.k + pad, y1: (app.offsetHeight - view.y) / view.k - pad,
  };
}
function clamp(p, b = bounds()) {
  p.x = Math.min(Math.max(p.x, b.x0), Math.max(b.x0, b.x1 - p.w));
  p.y = Math.min(Math.max(p.y, b.y0), Math.max(b.y0, b.y1 - p.h));
}
// After a resize: pull panes back onto the canvas, or, if that would stack them, back to the starting
// layout, which always fits.
function keepInside() {
  for (const p of panes) clamp(p);
  if (panes.some((a) => panes.some((b) => a !== b && touching(a, b)))) {
    for (const p of panes) { p.x = +p.el.dataset.x; p.y = +p.el.dataset.y; }
  }
  for (const p of panes) place(p);
  drawLinks();
}

// --- wires: the curve from scenes.js, with a dot at each end like the app draws -------------
const NS = "http://www.w3.org/2000/svg";
const links = LINKS.filter((pair) => pair.every((id) => byId[id])).map(([a, b]) => {
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

// The pane's drawn position, so wires follow panes while they glide.
function paintPane(p, x, y) {
  p.drawn = { x, y, w: p.w, h: p.h };
  p.el.style.transform = `translate(${x}px, ${y}px)`;
}
function place(p) {
  stopAnimation(p);
  paintPane(p, p.x, p.y);
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

// Lays the other panes out around `dragged`, each starting from its spot in `home`. Every order of the
// other panes is tried (four panes, so six orders) and the one that moves them least wins. Returns
// false, changing nothing, when no order leaves every pane clear: the dragged pane is over a spot with no
// room for the rest.
function makeRoom(dragged, home) {
  const b = bounds();
  let best = null;
  for (const order of orders(panes.filter((p) => p !== dragged))) {
    const settled = [dragged], spots = [];
    let cost = 0;
    for (const p of order) {
      const [hx, hy] = home.get(p);
      const spot = nearestFree(p, hx, hy, settled, b);
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
  const { x: fromX, y: fromY } = p.drawn;
  const { x, y } = p;
  animate(p, 500, (progress) => {
    paintPane(p, fromX + (x - fromX) * progress, fromY + (y - fromY) * progress);
    drawLinks();
  });
}

// --- dragging a pane by its header ----------------------------------------------------------
app.addEventListener("pointerdown", (event) => {
  const head = event.button === 0 && event.target.closest(".pane-head");
  if (!head) return; // everywhere else the pointer does nothing, and the wheel scrolls the page
  const pane = panes.find((p) => p.el.contains(head));
  const k = view.k * (app.getBoundingClientRect().width / app.offsetWidth); // screen px per world unit
  const sx = event.clientX, sy = event.clientY, ox = pane.x, oy = pane.y;
  // Every move lays the others out from where they were when the drag began, so panes you pass come back.
  const home = new Map(panes.map((p) => [p, [p.x, p.y]]));
  let good = home, fits = true; // the last layout with room for everything, and whether this one has room
  head.setPointerCapture(event.pointerId);
  pane.el.classList.add("lifted");
  stopAnimation(pane);
  const move = (e) => {
    pane.x = ox + (e.clientX - sx) / k; pane.y = oy + (e.clientY - sy) / k;
    clamp(pane);
    fits = makeRoom(pane, home);
    // With no room here the pane floats above the others, which hold the last layout that fitted.
    if (fits) good = new Map(panes.map((p) => [p, [p.x, p.y]]));
    else for (const p of panes) if (p !== pane) [p.x, p.y] = good.get(p);
    place(pane);
    for (const p of panes) if (p !== pane) glide(p);
    drawLinks();
  };
  const end = () => {
    if (!fits) { [pane.x, pane.y] = good.get(pane); glide(pane); } // dropped with no room: back it goes
    pane.el.classList.remove("lifted");
    head.removeEventListener("pointermove", move);
    head.removeEventListener("pointerup", end);
    head.removeEventListener("pointercancel", end);
  };
  head.addEventListener("pointermove", move);
  head.addEventListener("pointerup", end);
  head.addEventListener("pointercancel", end);
});

fit();
drawLinks();

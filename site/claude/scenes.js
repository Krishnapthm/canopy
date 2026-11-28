// Every picture of canopy on index.html is drawn as SVG from one set of parts: a pane with its header
// and state dot, terminal rows, a browser, an iPhone, the Harbor shop they show and wires. This file
// draws the hero canvas; shots.js builds the three feature shots from the same parts, so the page
// reads as one afternoon on one project. Text is real SVG text in the page's fonts (Geist, JetBrains
// Mono, the system face for Harbor), so it stays sharp at any size.
// Runs before canvas.js and shots.js: it fills the hero's panes ([data-pane]) and returns the parts.

const Scenes = (() => {
  const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  let count = 0;
  const uid = (prefix) => `${prefix}${++count}`;

  const CH = 7.8, ROW = 18, HEAD = 30; // JetBrains Mono at 13/18: one cell is 7.8 x 18
  const BLUE = "#1f5eff";               // Harbor's brand blue
  const PIN = "#2f7cf6";                // canopy's annotation blue

  // --- terminal rows ------------------------------------------------------------------------------
  // A row is a string: "{dim b|text}" colours a run, "{@rec.green}" draws, in one cell, a glyph the font
  // lacks (a terminal falls back to a system font for those). A row can also be { t, bg } for a
  // highlighted prompt ("umsg"), a selection ("sel") or a diff line ("add", "del").
  const GLYPH = {
    rec: (x, y) => `<circle cx="${x + 3.9}" cy="${y + 9.6}" r="3.1"/>`,
    elbow: (x, y) => `<path d="M${x + 2.6} ${y + 3.2}v7.6h5.2" fill="none" stroke-width="1"/>`,
    play: (x, y) => `<path d="M${x + 1.6} ${y + 5.7} ${x + 6.6} ${y + 9.5} ${x + 1.6} ${y + 13.3}z"/>`,
    caret: (x, y) => `<rect x="${x + .5}" y="${y + 2.5}" width="7" height="14" fill="none" stroke-width="1"/>`,
  };
  const BG = { umsg: "#1e1e1e", add: "#022800", del: "#3d0b0b", sel: "#26314a" };
  const classes = (list) => list.split(" ").map((c) => `c-${c}`).join(" ");

  function row(src, x, y) {
    let text = "", glyphs = "", col = 0, last = 0;
    const plain = (s) => { text += esc(s); col += [...s].length; };
    for (const m of src.matchAll(/\{@(\w+)(?:\.([\w]+))?\}|\{([\w ]+)\|([^}]*)\}/g)) {
      plain(src.slice(last, m.index));
      last = m.index + m[0].length;
      if (m[1]) {
        glyphs += `<g class="gl ${m[2] ? classes(m[2]) : ""}">${GLYPH[m[1]](x + col * CH, y)}</g>`;
        text += " "; col++;
      } else {
        text += `<tspan class="${classes(m[3])}">${esc(m[4])}</tspan>`;
        col += [...m[4]].length;
      }
    }
    plain(src.slice(last));
    return { svg: `<text x="${x}" y="${y + 13}" xml:space="preserve">${text}</text>${glyphs}`, cols: col };
  }

  // Rows from (x, y) down. `w` is the pane width, for full-width prompt highlights.
  function term(rows, { x = 16, y = 10, w }) {
    let out = "";
    rows.forEach((r, i) => {
      const { t, bg } = typeof r === "string" ? { t: r } : r;
      const top = y + i * ROW;
      const { svg, cols } = row(t, x, top);
      if (bg === "umsg" || bg === "sel") out += `<rect y="${top}" width="${w}" height="${ROW}" fill="${BG[bg]}"/>`;
      else if (bg) out += `<rect x="${x}" y="${top}" width="${cols * CH}" height="${ROW}" fill="${BG[bg]}"/>`;
      out += svg;
    });
    return `<g class="sv-mono">${out}</g>`;
  }
  // Rows that sit at the bottom of a pane, like a prompt.
  const termBottom = (rows, w, h, pad = 8) => term(rows, { y: h - pad - rows.length * ROW, w });

  // The status line shows the current step immediately and advances while the pane is visible.
  const activity = (steps, rows, color) => `<g data-working class="sv-mono">
      ${steps.map((step) => `<desc data-step>${esc(step)}</desc>`).join("")}
      <text class="c-${color}" x="16" y="${23 + rows.length * ROW}" data-spin>✻</text>
      <text x="39" y="${23 + rows.length * ROW}" xml:space="preserve"><tspan class="c-${color}" data-verb>${esc(steps[0])}</tspan><tspan class="c-dim"> (<tspan data-secs>12</tspan>s)</tspan></text></g>`;

  const working = (rows, steps) => (w, h) => {
    const rule = `{rule|${"─".repeat(Math.floor((w - 32) / CH))}}`;
    return term(rows, { w }) + activity(steps, rows, "clawd") + termBottom([rule, "{hi|❯} {@caret}", rule,
      "  {@play.yellow}{@play.yellow}{yellow| auto mode on} {dim|(shift+tab to cycle)}"], w, h);
  };

  const PAD = { t: "", bg: "umsg" };
  const codex = (rows, steps) => (w, h) => term(rows, { w }) + activity(steps, rows, "cyan") +
    termBottom([PAD, { t: "{b|›} {dim|Ask Codex to do anything}", bg: "umsg" }, PAD,
      "  {dim|Approval: automatic review}", "  {dim|gpt-6.2-sol medium · ~/harbor}"], w, h);

  // --- pane ---------------------------------------------------------------------------------------
  // The app's pane: #0c0c0c body, a 30px header with icon, title and state dot. `dot` is idle, work,
  // wait (orange) or done (green); null leaves it out.
  const ICON = {
    claude: `<use href="#i-claude" x="12" y="8" width="14" height="14"/>`,
    codex: `<use href="#i-codex" x="12" y="8" width="14" height="14"/>`,
    globe: `<use class="sv-ic" href="#i-globe" x="12" y="8" width="14" height="14"/>`,
    folder: `<use class="sv-ic" href="#i-folder" x="12" y="8" width="14" height="14"/>`,
  };
  function pane({ x = 0, y = 0, w, h, title, sub = "", icon, dot = "idle", body }) {
    const clip = uid("pane");
    return `<svg class="sv-pane" x="${x}" y="${y}" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" overflow="visible">
      <clipPath id="${clip}"><rect width="${w}" height="${h}" rx="6"/></clipPath>
      <g clip-path="url(#${clip})"><rect width="${w}" height="${h}" fill="#0c0c0c"/>
        <svg y="${HEAD}" width="${w}" height="${h - HEAD}" overflow="hidden">${body(w, h - HEAD)}</svg></g>
      <rect data-outline x=".5" y=".5" width="${w - 1}" height="${h - 1}" rx="6" fill="none" stroke="#fff" stroke-opacity=".11"/>
      <g class="pane-head"><rect width="${w}" height="${HEAD}" fill="transparent"/>${ICON[icon]}
        <text class="sv-ui" x="34" y="19.5">${esc(title)}${sub ? `<tspan class="c-dim" dx="8">${esc(sub)}</tspan>` : ""}</text>
        ${dot ? `<circle data-dot class="dot ${dot}" cx="${w - 15}" cy="15" r="5"/>` : ""}</g>
    </svg>`;
  }

  // --- wires --------------------------------------------------------------------------------------
  // A wire leaves the middle of one box's side and enters the facing side of the other, level at both
  // ends, so it always hangs in one long S, as the app draws its edges. Boxes that share no clear
  // column (stacked, as on a phone) get the same S turned on its side, bottom to top.
  function wirePath(A, B) {
    const sideBySide = B.x >= A.x + A.w - 40 || A.x >= B.x + B.w - 40;
    let p1, p2, c1, c2;
    if (sideBySide) {
      const flip = B.x < A.x;
      p1 = [flip ? A.x : A.x + A.w, A.y + A.h / 2];
      p2 = [flip ? B.x + B.w : B.x, B.y + B.h / 2];
      const pull = Math.max(Math.abs(p2[0] - p1[0]) / 2, 60) * (flip ? -1 : 1);
      c1 = [p1[0] + pull, p1[1]]; c2 = [p2[0] - pull, p2[1]];
    } else {
      const flip = B.y < A.y;
      p1 = [A.x + A.w / 2, flip ? A.y : A.y + A.h];
      p2 = [B.x + B.w / 2, flip ? B.y + B.h : B.y];
      const pull = Math.max(Math.abs(p2[1] - p1[1]) / 2, 60) * (flip ? -1 : 1);
      c1 = [p1[0], p1[1] + pull]; c2 = [p2[0], p2[1] - pull];
    }
    return { d: `M${p1}C${c1} ${c2} ${p2}`, p1, p2 };
  }
  // A drawn wire, with a dot at each end as the app draws them.
  function wire(A, B) {
    const { d, p1, p2 } = wirePath(A, B);
    return `<path class="wire" d="${d}"/><circle class="wire-end" cx="${p1[0]}" cy="${p1[1]}" r="3.5"/><circle class="wire-end" cx="${p2[0]}" cy="${p2[1]}" r="3.5"/>`;
  }


  // --- Harbor: the coffee shop the agents are building ----------------------------------------------
  const tx = (x, y, s, { size = 14, weight = 400, fill = "#17171a", anchor = "start", cls = "" } = {}) =>
    `<text class="sv-app ${cls}" x="${x}" y="${y}" font-size="${size}" font-weight="${weight}" fill="${fill}" text-anchor="${anchor}">${esc(s)}</text>`;
  const GREY = "#6e6e73";

  // A bag of Harbor coffee on its tile: kraft pouch, folded top, a coloured label.
  const bag = (x, y, s, label) => `<g transform="translate(${x} ${y}) scale(${s / 72})">
    <rect width="72" height="72" rx="12" fill="#f3eee6"/>
    <path d="M23 17h26l4 43a2 2 0 0 1-2 2H21a2 2 0 0 1-2-2z" fill="#cfae84"/>
    <path d="M23 17h26l.6 7H22.4z" fill="#b8936a"/><path d="M36 17v7" stroke="#a7825a" stroke-width="1"/>
    <rect x="25" y="32" width="22" height="20" rx="2.5" fill="${label}"/>
    <path d="M29 39h14M29 44.5h9" stroke="#fff" stroke-opacity=".85" stroke-width="1.8" stroke-linecap="round"/>
    <path d="M21 58q15 3 30 0" stroke="#000" stroke-opacity=".08" stroke-width="3" fill="none"/></g>`;
  const ITEMS = [
    { name: "Kona Sunrise", note: "Light roast · 250 g · whole bean", price: "$18.00", label: "#e5734f" },
    { name: "Night Shift", note: "Dark roast · 500 g · espresso grind", price: "$22.00", label: "#24304d" },
  ];
  // canopy's annotation marks: a numbered pin at (x, y).
  const pin = (x, y, k) => `<circle cx="${x}" cy="${y}" r="12" fill="${PIN}"/>${tx(x, y + 4.5, k, { size: 12.5, weight: 650, fill: "#fff", anchor: "middle" })}`;

  const harborHeader = (subscriptions = false) => `<rect width="760" height="900" fill="#fff"/>
      <use href="#harbor-mark" x="28" y="19" width="19" height="19" fill="none" stroke="${BLUE}" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
      ${tx(53, 34, "Harbor", { size: 16, weight: 650 })}
      ${tx(140, 33, "Shop", { size: 13.5, fill: GREY })}${tx(190, 33, "Subscriptions", { size: 13.5, weight: subscriptions ? 650 : 400, fill: subscriptions ? BLUE : GREY })}${tx(296, 33, "Journal", { size: 13.5, fill: GREY })}
      <path d="M697 22h16l-1.4 14h-13.2z M701 22a4 4 0 0 1 8 0" fill="none" stroke="#17171a" stroke-width="1.6" stroke-linejoin="round"/>
      <circle cx="716" cy="21" r="7.5" fill="${BLUE}"/>${tx(716, 25, "2", { size: 10.5, weight: 650, fill: "#fff", anchor: "middle" })}
      <rect y="55.5" width="760" height="1" fill="#ececf0"/>`;

  // The bag page on the web, 760 wide, with the two annotations canopy draws over it.
  function harborWeb() {
    let s = `${harborHeader()}
      ${tx(28, 100, "Your bag", { size: 27, weight: 680, cls: "tight" })}
      ${tx(28, 123, "2 items · ships Monday from Portland", { size: 14, fill: GREY })}`;
    ITEMS.forEach((it, i) => {
      const y = 150 + i * 112;
      s += `${bag(28, y + 4, 84, it.label)}
        ${tx(128, y + 26, it.name, { size: 16, weight: 600 })}${tx(462, y + 26, it.price, { size: 15, weight: 550, anchor: "end" })}
        ${tx(128, y + 47, it.note, { size: 13.5, fill: GREY })}
        <rect x="128.5" y="${y + 60.5}" width="92" height="28" rx="14" fill="none" stroke="#dcdce2"/>
        ${tx(143, y + 79, "−", { size: 15, fill: GREY })}${tx(174.5, y + 79, "1", { size: 13.5, weight: 550, anchor: "middle" })}${tx(206, y + 79, "+", { size: 15, fill: GREY, anchor: "end" })}
        ${tx(462, y + 79, "Remove", { size: 13, fill: GREY, anchor: "end" })}
        ${i === 0 ? `<rect x="28" y="${y + 104}" width="434" height="1" fill="#ececf0"/>` : ""}`;
    });
    // Free shipping: $40 of $50.
    s += `<rect x="28" y="390" width="434" height="66" rx="12" fill="#f5f6f8"/>
      ${tx(46, 416, "You're $10.00 away from free shipping", { size: 14, weight: 550 })}
      <rect x="46" y="430" width="398" height="7" rx="3.5" fill="#e2e4ea"/><rect x="46" y="430" width="318" height="7" rx="3.5" fill="${BLUE}"/>`;
    // Order summary.
    const sx = 494, sw = 238;
    s += `<rect x="${sx + .5}" y="150.5" width="${sw}" height="296" rx="14" fill="#fff" stroke="#e6e6eb"/>
      ${tx(sx + 20, 182, "Order summary", { size: 15.5, weight: 620 })}
      ${[["Subtotal", "$40.00"], ["Shipping", "$4.90"], ["Tax", "$3.30"]].map(([k, v], i) =>
        tx(sx + 20, 214 + i * 26, k, { size: 14, fill: GREY }) + tx(sx + sw - 20, 214 + i * 26, v, { size: 14, anchor: "end" })).join("")}
      <rect x="${sx + 20.5}" y="284.5" width="${sw - 41}" height="36" rx="9" fill="#fff" stroke="#dcdce2"/>
      ${tx(sx + 34, 307, "Promo code", { size: 13.5, fill: "#a1a1a8" })}${tx(sx + sw - 34, 307, "Apply", { size: 13.5, weight: 600, fill: BLUE, anchor: "end" })}
      <rect x="${sx + 20}" y="336" width="${sw - 40}" height="1" fill="#ececf0"/>
      ${tx(sx + 20, 366, "Total", { size: 16, weight: 650 })}${tx(sx + sw - 20, 366, "$48.20", { size: 16, weight: 650, anchor: "end" })}
      <rect x="${sx + 20}" y="384" width="${sw - 40}" height="44" rx="11" fill="#17171a"/>
      ${tx(sx + sw / 2, 411, "Checkout", { size: 15, weight: 600, fill: "#fff", anchor: "middle" })}`;
    // 1: an element ring on the promo field. 2: a pen stroke around the shipping bar.
    s += `<rect x="${sx + 15}" y="279" width="${sw - 30}" height="46" rx="12" fill="${PIN}" fill-opacity=".06" stroke="${PIN}" stroke-width="2"/>
      <path d="M24 402c-6-26 70-30 210-27 140 2 238 2 236 30 1 34-90 58-236 57C90 461 34 452 26 420c-1-10 6-18 22-21" fill="none" stroke="${PIN}" stroke-width="2.6" stroke-linecap="round"/>
      ${pin(sx + 15, 279, 1)}${pin(24, 386, 2)}`;
    return s;
  }

  // A full desktop preview of Harbor's other route, using the same storefront header and product art.
  function harborSubscriptions() {
    return `${harborHeader(true)}
      <rect x="28" y="76" width="704" height="168" rx="14" fill="#f3eee6"/>
      ${tx(53, 116, "Harbor Coffee Club", { size: 13, weight: 650, fill: GREY })}
      ${tx(53, 158, "Good coffee, on repeat.", { size: 30, weight: 700, cls: "tight" })}
      ${tx(53, 186, "Freshly roasted in Portland. Pause or skip anytime.", { size: 14, fill: GREY })}
      ${bag(524, 97, 118, ITEMS[0].label)}${bag(600, 122, 103, ITEMS[1].label)}
      ${tx(28, 280, "Choose your plan", { size: 19, weight: 650 })}
      ${[["Roaster’s Choice", "A rotating single origin", "$18 / delivery"], ["Espresso Club", "Our house espresso blend", "$22 / delivery"]].map(([name, note, price], i) => {
        const x = 28 + i * 360;
        return `<rect x="${x}" y="298" width="344" height="146" rx="12" fill="#fff" stroke="#dcdce2"/>
          ${tx(x + 18, 328, name, { size: 16, weight: 650 })}
          ${tx(x + 18, 351, note, { size: 13, fill: GREY })}
          ${tx(x + 18, 406, price, { size: 14, weight: 650 })}
          <rect x="${x + 213}" y="381" width="114" height="38" rx="8" fill="${BLUE}"/>
          ${tx(x + 270, 405, "Choose plan", { size: 13, weight: 600, fill: "#fff", anchor: "middle" })}`;
      }).join("")}`;
  }

  // Liquid glass, as iOS 26 draws its floating controls: a tinted body, a sheen across the top half and
  // a bright rim that fades down the sides. `shape` is the rect or circle's geometry attributes.
  const glass = (tag, shape, tint = "#fff", opacity = .72) => `<g filter="url(#glass-shadow)"><${tag} ${shape} fill="${tint}" fill-opacity="${opacity}"/></g>
    <${tag} ${shape} fill="url(#glass-sheen)"/><${tag} ${shape} fill="none" stroke="url(#glass-rim)" stroke-width="1.2"/>`;

  // Harbor's iPhone app at checkout, 402 x 874 points, in iOS 26's liquid glass: the content runs under
  // a glass back button and a solid Place order button, behind soft edge fades. The scroll group, the
  // button and the "placed" screen carry data attributes so the loop below can scroll, press and confirm.
  // The status bar and the Dynamic Island are drawn last, so nothing ever covers them. Radios, delivery
  // prices and the totals carry data attributes so the feature shot (shots.js) can tap through checkout:
  // a radio is its ring with its blue fill on top ([data-on]), shown or hidden.
  function harborCheckout() {
    const card = (y, h) => `<rect x="16" y="${y}" width="370" height="${h}" rx="22" fill="#fff"/>`;
    const label = (y, s) => tx(32, y, s, { size: 13, fill: GREY, cls: "caps" });
    const radio = (name, x, y, on) => `<g data-radio="${name}"><circle cx="${x}" cy="${y}" r="10.25" fill="none" stroke="#c7c7cc" stroke-width="1.5"/>
      <g data-on${on ? "" : ` opacity="0"`}><circle cx="${x}" cy="${y}" r="11" fill="${BLUE}"/><circle cx="${x}" cy="${y}" r="4.2" fill="#fff"/></g></g>`;
    const line = (y) => `<rect x="32" y="${y}" width="354" height=".7" fill="#d9d9de"/>`;
    const value = (name, x, y, s, opts) => tx(x, y, s, opts).replace("<text", `<text data-${name}`);
    const content = `
      ${tx(20, 160, "Checkout", { size: 34, weight: 700, cls: "tight" })}
      ${label(202, "SHIP TO")}${card(212, 76)}
      ${tx(32, 242, "Maya Lin", { size: 17, weight: 600 })}${tx(32, 266, "418 Harbor St, Apt 3 · Portland, OR", { size: 15, fill: GREY })}
      <path d="M364 243l6 7-6 7" fill="none" stroke="#c4c4c9" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      ${label(326, "DELIVERY")}${card(336, 124)}
      ${tx(32, 368, "Standard", { size: 17, weight: 500 })}${tx(32, 390, "Arrives Monday", { size: 14, fill: GREY })}${value("price", 332, 379, "$4.90", { size: 16, anchor: "end" })}${radio("standard", 356, 374, true)}
      ${line(398)}
      ${tx(32, 430, "Express", { size: 17, weight: 500 })}${tx(32, 452, "Arrives Saturday", { size: 14, fill: GREY })}${value("price", 332, 441, "$12.00", { size: 16, anchor: "end" })}${radio("express", 356, 436, false)}
      ${label(498, "PAYMENT")}${card(508, 124)}
      <rect x="32" y="525" width="48" height="31" rx="6" fill="#000"/>${tx(56, 546, "Pay", { size: 14, weight: 600, fill: "#fff", anchor: "middle" })}
      ${tx(94, 546, "Apple Pay", { size: 17, weight: 500 })}${radio("applepay", 356, 540, true)}
      ${line(570)}
      <rect x="32" y="586" width="48" height="31" rx="6" fill="#1a1f71"/>${tx(56, 607, "VISA", { size: 12.5, weight: 800, fill: "#fff", anchor: "middle", cls: "italic" })}
      ${tx(94, 607, "Visa •••• 4242", { size: 17, weight: 500 })}${radio("visa", 356, 602, false)}
      ${label(670, "SUMMARY")}${card(680, 252)}
      ${ITEMS.map((it, i) => `${bag(32, 696 + i * 62, 48, it.label)}${tx(92, 717 + i * 62, it.name, { size: 16, weight: 600 })}${tx(92, 737 + i * 62, it.note.split(" · ").slice(0, 2).join(" · "), { size: 13.5, fill: GREY })}${tx(370, 717 + i * 62, it.price, { size: 16, anchor: "end" })}`).join("")}
      ${line(820)}
      ${[["Subtotal", "$40.00"], ["Shipping", "$4.90"], ["Tax", "$3.30"]].map(([k, v], i) => tx(32, 844 + i * 22, k, { size: 15, fill: GREY }) + value(k.toLowerCase(), 370, 844 + i * 22, v, { size: 15, anchor: "end" })).join("")}
      ${tx(32, 920, "Total", { size: 17, weight: 650 })}${value("total", 370, 920, "$48.20", { size: 17, weight: 650, anchor: "end" })}`;
    const chrome = `${tx(52, 37, "9:41", { size: 17, weight: 600, fill: "#000" })}
      <rect x="139" y="11" width="124" height="36" rx="18" fill="#000"/>
      <use href="#ios-signal" x="290" y="26" width="19" height="12" fill="#000"/><use href="#ios-wifi" x="315" y="26" width="17" height="12" fill="#000"/><use href="#ios-battery" x="338" y="25.5" width="28" height="13" fill="#000" color="#000"/>
      <rect x="131" y="860" width="140" height="5" rx="2.5" fill="#000"/>`;
    return `<rect width="402" height="874" fill="#f2f2f7"/>
      <g data-scroll>${content}</g>
      <rect width="402" height="124" fill="url(#ios-fade-top)"/><rect y="724" width="402" height="150" fill="url(#ios-fade-bottom)"/>
      ${glass("circle", `cx="40" cy="84" r="22"`)}
      <path d="M43 75l-9 9 9 9" fill="none" stroke="#000" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>
      <g data-btn><rect x="16" y="776" width="370" height="54" rx="27" fill="${BLUE}"/>${tx(201, 809, "Place order · $48.20", { size: 17, weight: 600, fill: "#fff", anchor: "middle" })}</g>
      <g data-done opacity="0"><rect width="402" height="874" fill="#fff"/>
        <circle cx="201" cy="340" r="44" fill="#34c759"/><path d="M182 341l13 13 25-27" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/>
        ${tx(201, 428, "Order placed", { size: 28, weight: 700, anchor: "middle", cls: "tight" })}
        ${tx(201, 458, "HB-2041 · arriving Monday", { size: 17, fill: GREY, anchor: "middle" })}
        <rect x="16" y="776" width="370" height="54" rx="27" fill="#e9e9ef"/>${tx(201, 809, "Done", { size: 17, weight: 600, fill: BLUE, anchor: "middle" })}</g>
      ${chrome}`;
  }

  // --- browser and phone ---------------------------------------------------------------
  const browser = (url, page, annotations = true) => (w, h) => {
    const [host, path] = url.split(/(?=\/)/);
    const urlW = w - 90 - (annotations ? 156 : 0);
    const vbH = (h - 38) * 760 / w; // the page is laid out at 760 wide and scaled to the pane
    return `<g class="sv-ic"><use href="#i-back" x="12" y="9" width="16" height="16"/><use href="#i-fwd" x="34" y="9" width="16" height="16" opacity=".35"/><use href="#i-reload" x="56" y="9" width="16" height="16"/></g>
      <rect x="80" y="4" width="${urlW}" height="26" rx="7" fill="#1c1c1c"/>
      <text class="sv-ui" x="90" y="21"><tspan class="c-dim">${esc(host)}</tspan>${esc(path)}</text>
      ${annotations ? `<rect x="${w - 170}" y="4" width="84" height="26" rx="7" fill="#2a2a2a"/>
      <use class="sv-ic" href="#i-pen" x="${w - 162}" y="10.5" width="13" height="13"/><text class="sv-ui" x="${w - 145}" y="21">Annotate</text>
      <rect x="${w - 80}" y="4" width="70" height="26" rx="7" fill="${PIN}"/><text class="sv-ui" x="${w - 45}" y="21" text-anchor="middle">Send (2)</text>` : ""}
      <svg y="38" width="${w}" height="${h - 38}" viewBox="0 0 760 ${vbH}" preserveAspectRatio="xMinYMin slice">${page}</svg>`;
  };

  // iPhone 17 Pro on its own, 1 unit per point: a 402 x 874 screen in a titanium band, as large as
  // (w, h) allows. The simulator pane shows just the device, so the device is the pane: `drag` makes
  // the whole phone its handle. `checkout` plays the CSS checkout loop on it (the hero's phone); the
  // feature shot drives its own, and draws over the screen through [data-screen], in points.
  const phone = (screen, { drag = false, checkout = true } = {}) => (w, h) => {
    const pw = Math.min(w, h * 432 / 904), ph = pw * 904 / 432;
    const ti = uid("ti"), sc = uid("scr");
    return `<svg ${drag ? `class="pane-head"` : ""} x="${(w - pw) / 2}" y="${(h - ph) / 2}" width="${pw}" height="${ph}" viewBox="0 0 432 904"${checkout ? " data-checkout" : ""}>
      <defs><linearGradient id="${ti}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#6a6a70"/><stop offset=".45" stop-color="#2d2d31"/><stop offset="1" stop-color="#4c4c52"/></linearGradient>
        <clipPath id="${sc}"><rect width="402" height="874" rx="55"/></clipPath></defs>
      <rect x="0" y="178" width="4" height="34" rx="1.5" fill="#3a3a3e"/><rect x="0" y="236" width="4" height="62" rx="1.5" fill="#3a3a3e"/>
      <rect x="0" y="312" width="4" height="62" rx="1.5" fill="#3a3a3e"/><rect x="428" y="262" width="4" height="96" rx="1.5" fill="#3a3a3e"/>
      <rect x="2.5" y="0" width="427" height="904" rx="68" fill="url(#${ti})"/>
      <rect x="5" y="2.5" width="422" height="899" rx="65.5" fill="#0a0a0b"/>
      <svg data-screen x="15" y="15" width="402" height="874"><g clip-path="url(#${sc})">${screen}</g></svg>
    </svg>`;
  };

  // --- the session ----------------------------------------------------------------------------------
  // Harbor has checkout, subscriptions, and inventory work in flight on the same canvas.
  // The feature shots (shots.js) use the original checkout session.
  const CHECKOUT = [
    { t: "{dim|❯} sync checkout on the web and iPhone", bg: "umsg" },
    "",
    "{@rec.green} {b|Read} 2 annotations on localhost:5173/bag",
    "  {@elbow.dim} Promo field: move it above the total.",
    "  {@elbow.dim} Shipping bar: count discounts first.",
    "{@rec.green} {b|Update}(src/routes/bag.tsx)",
    "  {@elbow.dim} Added 18 lines, removed 6 lines.",
    "{@rec.green} {b|Update}(CheckoutSummary.swift)",
    "  {@elbow.dim} Free shipping now uses the net subtotal.",
    "{@rec.green} {b|Bash}(canopy pane devices)",
    "  {@elbow.dim} iPhone 17 Pro · Booted",
    "{@rec.green} {b|Bash}(npm test -- checkout)",
    "  {@elbow.dim} 8 passed, including the $50 threshold.",
    "",
  ];
  const PROMO = [
    { t: "{b|›} add regression tests for promo codes", bg: "umsg" },
    "",
    "{green|•} {b|Explored}",
    "  {dim|└ Read src/lib/promo.ts, promo.test.ts}",
    "{green|•} Expired and stacked codes are untested.",
    "  The API caps each discount at 10%.",
    "{green|•} {b|Edited} src/lib/promo.test.ts {green|+42}",
    "  {dim|└ Fixed the test clock at midnight UTC.}",
    "{green|•} {b|Ran} npm test -- promo",
    "  {dim|└} {green|9 passed} {dim|(380ms)}",
    "{green|•} {b|Edited} tests/checkout.spec.ts {green|+16}",
    "  {dim|└ Covers a valid code after a rejected one.}",
    "",
  ];
  // The hero's panes, keyed by the ids in index.html. A device renders without pane chrome.
  const HERO = {
    t1: { title: "Claude Code", sub: "checkout", icon: "claude", dot: "work",
      body: working(CHECKOUT, ["Testing iPhone checkout", "Checking the discount in Safari", "Comparing shipping totals"]) },
    codex: { title: "Codex", sub: "promo tests", icon: "codex", dot: "work",
      body: codex(PROMO, ["Running the browser regression", "Checking the test trace", "Verifying the expiry boundary"]) },
    browser: { title: "Harbor · Your bag", icon: "globe", body: browser("localhost:5173/bag", harborWeb()) },
    sim: { device: harborCheckout() },
    subscriptions: { title: "Harbor · Subscriptions", icon: "globe", dot: null,
      body: browser("localhost:5173/subscriptions", harborSubscriptions(), false) },
    "subscription-agent": { title: "Claude Code", sub: "subscriptions", icon: "claude", dot: "work", body: working([
      { t: "{dim|❯} Build the coffee club plans page", bg: "umsg" },
      "",
      "{@rec.green} {b|Read}(src/lib/plans.ts)",
      "  {@elbow.dim} Two plans, billed per delivery.",
      "{@rec.green} {b|Update}(src/routes/subscriptions.tsx)",
      "  {@elbow.dim} Reused the shop header and product tiles.",
      '{@rec.green} {b|Bash}(hc send review "Check plan pricing.")',
      "  {@elbow.dim} Sent to the reviewer.",
      "{@rec.green} {b|Bash}(npm test -- subscriptions)",
      "  {@elbow.dim} 4 passed, including the pause flow.",
    ], ["Checking the desktop preview", "Testing the plan buttons", "Comparing the mobile layout"]) },
    "subscription-review": { title: "Codex", sub: "plan review", icon: "codex", dot: "work", body: codex([
      { t: "{b|›} Check plan pricing before checkout", bg: "umsg" },
      "",
      "{green|•} {b|Explored} src/lib/plans.ts",
      "  {dim|└ $18 single origin, $22 espresso.}",
      "{green|•} {b|Compared} the card and checkout prices.",
      "  Both match. Shipping is added at checkout.",
      "{green|•} {b|Edited} subscriptions.spec.ts {green|+24}",
      "  {dim|└ Assert price survives plan selection.}",
    ], ["Running the pricing check", "Checking both plan fixtures", "Preparing feedback for Claude"]) },
    inventory: { title: "Claude Code", sub: "inventory", icon: "claude", dot: "work", body: working([
      { t: "{dim|❯} show when a roast has sold out", bg: "umsg" },
      "",
      "{@rec.green} {b|Read}(src/lib/inventory.ts)",
      "  {@elbow.dim} Stock is tracked per bag size.",
      "{@rec.green} {b|Read}(src/components/ProductTile.tsx)",
      "  {@elbow.dim} The buy button ignores the selected size.",
      "{@rec.green} {b|Update}(src/components/ProductTile.tsx)",
      "  {@elbow.dim} Disable purchase for the sold-out size.",
      "{@rec.green} {b|Update}(tests/fixtures/inventory.json)",
      "  {@elbow.dim} Night Shift 500 g is sold out.",
      "  {@elbow.dim} Keep the 250 g variant in stock.",
      "{@rec.green} {b|Bash}(npm test -- inventory)",
      "  {@elbow.dim} Stock and restock cases pass.",
    ], ["Testing the sold-out fixture", "Checking keyboard navigation", "Trying the restock flow"]) },
    "inventory-tests": { title: "Terminal", sub: "inventory tests", icon: "folder", dot: "work", body: (w) => {
      const rows = [
        "{green|❯} npm test -- inventory",
        "",
        " {dim|RUN  src/lib/inventory.test.ts}",
        " {green|✓} rejects sold-out variants",
        " {green|✓} leaves other sizes available",
        " {green|✓} restores checkout on restock",
        "",
        " Test Files  {green|1 passed} (1)",
        "      Tests  {green|3 passed} (3)",
        "",
        "{green|❯} npx playwright test inventory",
        "",
        " Running 2 tests using 1 worker",
        " {green|✓} 1. sold-out size is unavailable (1.2s)",
      ];
      return term(rows, { w }) + activity(["Testing restock in Chromium", "Waiting for the stock update"], rows, "green");
    } },
  };

  // Each visible pane updates its own status. Offscreen panes keep their last frame.
  const GLYPHS = ["·", "✢", "✳", "✶", "✻", "✶", "✳", "✢"];
  function startWorking(svg) {
    const spin = svg.querySelector("[data-spin]"), verb = svg.querySelector("[data-verb]"), secs = svg.querySelector("[data-secs]");
    const steps = [...svg.querySelectorAll("[data-step]")].map((step) => step.textContent);
    let frame = 0, timer;
    const tick = () => {
      frame++;
      spin.textContent = GLYPHS[frame % GLYPHS.length];
      if (frame % 8 === 0) secs.textContent = 12 + (frame / 8 | 0);
      if (frame % 32 === 0) verb.textContent = steps[(frame / 32) % steps.length];
    };
    new IntersectionObserver(([e]) => { clearInterval(timer); if (e.isIntersecting) timer = setInterval(tick, 120); }).observe(svg);
  }

  // --- render ----------------------------------------------------------------------------------------
  // Shared paint: the popover shadow, liquid glass and the iOS scroll edge fades.
  document.body.insertAdjacentHTML("afterbegin", `<svg width="0" height="0" class="defs" aria-hidden="true">
    <filter id="pop-shadow" x="-20%" y="-30%" width="140%" height="180%"><feDropShadow dx="0" dy="8" stdDeviation="10" flood-color="#000" flood-opacity=".28"/></filter>
    <filter id="glass-shadow" x="-30%" y="-30%" width="160%" height="190%"><feDropShadow dx="0" dy="6" stdDeviation="8" flood-color="#000" flood-opacity=".14"/></filter>
    <linearGradient id="glass-sheen" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity=".6"/><stop offset=".5" stop-color="#fff" stop-opacity="0"/></linearGradient>
    <linearGradient id="glass-rim" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fff"/><stop offset=".45" stop-color="#fff" stop-opacity=".2"/><stop offset="1" stop-color="#fff" stop-opacity=".75"/></linearGradient>
    <linearGradient id="ios-fade-top" x1="0" y1="0" x2="0" y2="1"><stop offset=".45" stop-color="#f2f2f7"/><stop offset="1" stop-color="#f2f2f7" stop-opacity="0"/></linearGradient>
    <linearGradient id="ios-fade-bottom" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#f2f2f7" stop-opacity="0"/><stop offset=".55" stop-color="#f2f2f7"/></linearGradient>
  </svg>`);
  for (const el of document.querySelectorAll("[data-pane]")) {
    const w = parseFloat(el.style.width), h = parseFloat(el.style.height), def = HERO[el.dataset.pane];
    el.innerHTML = `<svg class="sv" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" overflow="visible">${def.device ? phone(def.device, { drag: true })(w, h) : pane({ w, h, ...def })}</svg>`;
  }
  if (matchMedia("(prefers-reduced-motion: no-preference)").matches) {
    const checkout = new IntersectionObserver((entries) => {
      for (const { target, isIntersecting } of entries) target.classList.toggle("checkout-playing", isIntersecting);
    });
    document.querySelectorAll("[data-checkout]").forEach((phone) => checkout.observe(phone));
    document.querySelectorAll("[data-working]").forEach(startWorking);
  }

  // The parts, for the feature shots in shots.js.
  return { esc, uid, CH, ROW, HEAD, BG, PIN, row, term, pane, wirePath, wire, tx, harborCheckout, phone };
})();

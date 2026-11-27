// Every picture of canopy on claude.html, drawn as SVG from one set of parts: a pane with its header
// and state dot, terminal rows, a browser, an iPhone, the Harbor shop they show, the Files panel and
// wires. The hero canvas and the feature shots use the same parts and the same session, so the page
// reads as one afternoon on one project. Text is real SVG text in the page's fonts (Geist, JetBrains
// Mono, the system face for Harbor), so it stays sharp at any size.
// Runs before canvas.js: it fills the hero's panes ([data-pane]) and the shots ([data-scene]).

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

  // Claude Code: the transcript, then its prompt box and mode line pinned to the bottom.
  const claude = (rows, mode = "accept edits on") => (w, h) => {
    const rule = `{rule|${"─".repeat(Math.floor((w - 32) / CH))}}`;
    return term(rows, { w }) + termBottom([rule, "{hi|❯} {@caret}", rule,
      `  {@play.violet}{@play.violet}{violet| ${mode}} {dim|· Opus 5.5}`], w, h);
  };
  // Claude Code at work: the transcript, then its spinner line, animated by startWorking below.
  const working = (rows) => (w, h) => {
    const y = 10 + rows.length * ROW;
    return claude(rows)(w, h) + `<g data-working class="sv-mono">
      <text class="c-clawd" x="16" y="${y + 13}" data-spin>✻</text>
      <text x="39" y="${y + 13}" xml:space="preserve"><tspan class="c-clawd c-shimmer" data-verb>Percolating…</tspan><tspan class="c-dim"> (<tspan data-secs>12</tspan>s · esc to interrupt)</tspan></text></g>`;
  };
  // Codex: the transcript, then its composer and model line, unless it is waiting on an approval.
  const composer = (w, h) => termBottom([{ t: "{b|›} {dim|Ask Codex to do anything}", bg: "umsg" }, "", "  {dim|gpt-6.2-sol medium · ~/harbor}"], w, h);
  const codex = (rows, { waiting = false } = {}) => (w, h) => term(rows, { w }) + (waiting ? "" : composer(w, h));


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
      <rect x=".5" y=".5" width="${w - 1}" height="${h - 1}" rx="6" fill="none" stroke="#ffffff1c"/>
      <g class="pane-head"><rect width="${w}" height="${HEAD}" fill="transparent"/>${ICON[icon]}
        <text class="sv-ui" x="34" y="19.5">${esc(title)}${sub ? `<tspan class="c-dim" dx="8">${esc(sub)}</tspan>` : ""}</text>
        ${dot ? `<circle class="dot ${dot}" cx="${w - 15}" cy="15" r="5"/>` : ""}</g>
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
  // A drawn wire. `talk` sends a message dot along it, on a loop.
  function wire(A, B, { talk = false } = {}) {
    const { d, p1, p2 } = wirePath(A, B);
    const msg = (keys) => `<circle class="wire-msg" r="3"><animateMotion dur="2.4s" repeatCount="indefinite" path="${d}" keyPoints="${keys}" keyTimes="0;1" calcMode="spline" keySplines=".45 0 .55 1"/></circle>`;
    return `<path class="wire" d="${d}"/>${talk ? `<g data-talk>${msg("0;1")}</g>` : ""}<circle class="wire-end" cx="${p1[0]}" cy="${p1[1]}" r="3.5"/><circle class="wire-end" cx="${p2[0]}" cy="${p2[1]}" r="3.5"/>`;
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

  // The bag page on the web, 760 wide, with the two annotations canopy draws over it.
  function harborWeb() {
    let s = `<rect width="760" height="900" fill="#fff"/>
      <use href="#harbor-mark" x="28" y="19" width="19" height="19" fill="none" stroke="${BLUE}" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
      ${tx(53, 34, "Harbor", { size: 16, weight: 650 })}
      ${tx(140, 33, "Shop", { size: 13.5, fill: GREY })}${tx(190, 33, "Subscriptions", { size: 13.5, fill: GREY })}${tx(296, 33, "Journal", { size: 13.5, fill: GREY })}
      <path d="M697 22h16l-1.4 14h-13.2z M701 22a4 4 0 0 1 8 0" fill="none" stroke="#17171a" stroke-width="1.6" stroke-linejoin="round"/>
      <circle cx="716" cy="21" r="7.5" fill="${BLUE}"/>${tx(716, 25, "2", { size: 10.5, weight: 650, fill: "#fff", anchor: "middle" })}
      <rect y="55.5" width="760" height="1" fill="#ececf0"/>
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

  // Liquid glass, as iOS 26 draws its floating controls: a tinted body, a sheen across the top half and
  // a bright rim that fades down the sides. `shape` is the rect or circle's geometry attributes.
  const glass = (tag, shape, tint = "#fff", opacity = .72) => `<g filter="url(#glass-shadow)"><${tag} ${shape} fill="${tint}" fill-opacity="${opacity}"/></g>
    <${tag} ${shape} fill="url(#glass-sheen)"/><${tag} ${shape} fill="none" stroke="url(#glass-rim)" stroke-width="1.2"/>`;

  // Harbor's iPhone app at checkout, 402 x 874 points, in iOS 26's liquid glass: the content runs under
  // a glass back button and a solid Place order button, behind soft edge fades. The scroll group, the
  // button and the "placed" screen carry data attributes so the loop below can scroll, press and confirm.
  // The status bar and the Dynamic Island are drawn last, so nothing ever covers them. `annot` adds a
  // canopy annotation on the Express row.
  function harborCheckout({ annot = false } = {}) {
    const card = (y, h) => `<rect x="16" y="${y}" width="370" height="${h}" rx="22" fill="#fff"/>`;
    const label = (y, s) => tx(32, y, s, { size: 13, fill: GREY, cls: "caps" });
    const radio = (x, y, on) => on
      ? `<circle cx="${x}" cy="${y}" r="11" fill="${BLUE}"/><circle cx="${x}" cy="${y}" r="4.2" fill="#fff"/>`
      : `<circle cx="${x}" cy="${y}" r="10.25" fill="none" stroke="#c7c7cc" stroke-width="1.5"/>`;
    const line = (y) => `<rect x="32" y="${y}" width="354" height=".7" fill="#d9d9de"/>`;
    const content = `
      ${tx(20, 160, "Checkout", { size: 34, weight: 700, cls: "tight" })}
      ${label(202, "SHIP TO")}${card(212, 76)}
      ${tx(32, 242, "Maya Lin", { size: 17, weight: 600 })}${tx(32, 266, "418 Harbor St, Apt 3 · Portland, OR", { size: 15, fill: GREY })}
      <path d="M364 243l6 7-6 7" fill="none" stroke="#c4c4c9" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      ${label(326, "DELIVERY")}${card(336, 124)}
      ${tx(32, 368, "Standard", { size: 17, weight: 500 })}${tx(32, 390, "Arrives Monday", { size: 14, fill: GREY })}${tx(332, 379, "$4.90", { size: 16, anchor: "end" })}${radio(356, 374, true)}
      ${line(398)}
      ${tx(32, 430, "Express", { size: 17, weight: 500 })}${tx(32, 452, "Arrives Saturday", { size: 14, fill: GREY })}${tx(332, 441, "$12.00", { size: 16, anchor: "end" })}${radio(356, 436, false)}
      ${annot ? `<rect x="20" y="406" width="362" height="58" rx="16" fill="${PIN}" fill-opacity=".07" stroke="${PIN}" stroke-width="2.5"/>${pin(22, 408, 1)}` : ""}
      ${label(498, "PAYMENT")}${card(508, 124)}
      <rect x="32" y="525" width="48" height="31" rx="6" fill="#000"/>${tx(56, 546, "Pay", { size: 14, weight: 600, fill: "#fff", anchor: "middle" })}
      ${tx(94, 546, "Apple Pay", { size: 17, weight: 500 })}${radio(356, 540, true)}
      ${line(570)}
      <rect x="32" y="586" width="48" height="31" rx="6" fill="#1a1f71"/>${tx(56, 607, "VISA", { size: 12.5, weight: 800, fill: "#fff", anchor: "middle", cls: "italic" })}
      ${tx(94, 607, "Visa •••• 4242", { size: 17, weight: 500 })}${radio(356, 602, false)}
      ${label(670, "SUMMARY")}${card(680, 252)}
      ${ITEMS.map((it, i) => `${bag(32, 696 + i * 62, 48, it.label)}${tx(92, 717 + i * 62, it.name, { size: 16, weight: 600 })}${tx(92, 737 + i * 62, it.note.split(" · ").slice(0, 2).join(" · "), { size: 13.5, fill: GREY })}${tx(370, 717 + i * 62, it.price, { size: 16, anchor: "end" })}`).join("")}
      ${line(820)}
      ${[["Subtotal", "$40.00"], ["Shipping", "$4.90"], ["Tax", "$3.30"]].map(([k, v], i) => tx(32, 844 + i * 22, k, { size: 15, fill: GREY }) + tx(370, 844 + i * 22, v, { size: 15, anchor: "end" })).join("")}
      ${tx(32, 920, "Total", { size: 17, weight: 650 })}${tx(370, 920, "$48.20", { size: 17, weight: 650, anchor: "end" })}`;
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

  // --- browser, phone and Files panel ---------------------------------------------------------------
  const browser = (url, page) => (w, h) => {
    const [host, path] = url.split(/(?=\/)/);
    const urlW = w - 80 - 92 - 74;
    const vbH = (h - 38) * 760 / w; // the page is laid out at 760 wide and scaled to the pane
    return `<g class="sv-ic"><use href="#i-back" x="12" y="9" width="16" height="16"/><use href="#i-fwd" x="34" y="9" width="16" height="16" opacity=".35"/><use href="#i-reload" x="56" y="9" width="16" height="16"/></g>
      <rect x="80" y="4" width="${urlW}" height="26" rx="7" fill="#1c1c1c"/>
      <text class="sv-ui" x="90" y="21"><tspan class="c-dim">${esc(host)}</tspan>${esc(path)}</text>
      <rect x="${w - 170}" y="4" width="84" height="26" rx="7" fill="#2a2a2a"/>
      <use class="sv-ic" href="#i-pen" x="${w - 162}" y="10.5" width="13" height="13"/><text class="sv-ui" x="${w - 145}" y="21">Annotate</text>
      <rect x="${w - 80}" y="4" width="70" height="26" rx="7" fill="${PIN}"/><text class="sv-ui" x="${w - 45}" y="21" text-anchor="middle">Send (2)</text>
      <svg y="38" width="${w}" height="${h - 38}" viewBox="0 0 760 ${vbH}" preserveAspectRatio="xMinYMin slice">${page}</svg>`;
  };

  // iPhone 17 Pro on its own, 1 unit per point: a 402 x 874 screen in a titanium band, as large as
  // (w, h) allows. The simulator pane shows just the device, so the device is the pane: `drag` makes
  // the whole phone its handle.
  const phone = (screen, { drag = false } = {}) => (w, h) => {
    const pw = Math.min(w, h * 432 / 904), ph = pw * 904 / 432;
    const ti = uid("ti"), sc = uid("scr");
    return `<svg ${drag ? `class="pane-head"` : ""} x="${(w - pw) / 2}" y="${(h - ph) / 2}" width="${pw}" height="${ph}" viewBox="0 0 432 904" data-checkout>
      <defs><linearGradient id="${ti}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#6a6a70"/><stop offset=".45" stop-color="#2d2d31"/><stop offset="1" stop-color="#4c4c52"/></linearGradient>
        <clipPath id="${sc}"><rect width="402" height="874" rx="55"/></clipPath></defs>
      <rect x="0" y="178" width="4" height="34" rx="1.5" fill="#3a3a3e"/><rect x="0" y="236" width="4" height="62" rx="1.5" fill="#3a3a3e"/>
      <rect x="0" y="312" width="4" height="62" rx="1.5" fill="#3a3a3e"/><rect x="428" y="262" width="4" height="96" rx="1.5" fill="#3a3a3e"/>
      <rect x="2.5" y="0" width="427" height="904" rx="68" fill="url(#${ti})"/>
      <rect x="5" y="2.5" width="422" height="899" rx="65.5" fill="#0a0a0b"/>
      <svg x="15" y="15" width="402" height="874"><g clip-path="url(#${sc})">${screen}</g></svg>
    </svg>`;
  };

  // The Files panel: bag.tsx open in source with its Git change bars, a line quoted to the wired Claude
  // and a quick edit in progress, beside the file tree.
  const CODE = [
    [24, "{kw|export function} {fn|Summary}({ bag }: { bag: {type|Bag} }) {"],
    [25, "  {kw|const} total = {fn|useTotal}(bag);"],
    [26, "  {kw|return} ("],
    [27, "    <{fn|Card} {type|title}={amber|\"Order summary\"}>"],
    [28, "      <{fn|Row} {type|label}={amber|\"Subtotal\"} {type|value}={bag.subtotal} />"],
    [29, "      <{fn|Row} {type|label}={amber|\"Shipping\"} {type|value}={bag.shipping} />"],
    [30, "      <{fn|PromoField} {type|onApply}={bag.applyCode} />", "add"],
    [31, "      <{fn|Divider} />", "add"],
    [32, "      <{fn|Row} {type|label}={amber|\"Total\"} {type|value}={total} {type|strong} />", "sel"],
    [33, "      <{fn|Button} {type|onClick}={checkout}>Place order", "edit"],
    [34, "      </{fn|Button}>"],
    [35, "    </{fn|Card}>"],
    [36, "  );"],
    [37, "}"],
  ];
  const TREE = [
    [0, "harbor", "open"], [1, "ios", "shut"], [1, "src", "open"], [2, "api", "shut"], [2, "lib", "shut"], [2, "routes", "open"],
    [3, "bag.tsx", "M"], [3, "checkout.tsx"], [3, "index.tsx"], [1, ".env"], [1, ".gitignore"], [1, "package.json"], [1, "README.md"],
  ];
  const files = (w, h) => {
    const tw = 196, ew = w - tw, top = 42; // tree width, editor width, first code row
    const ui = (x, y, s, cls = "") => `<text class="sv-ui ${cls}" x="${x}" y="${y}">${esc(s)}</text>`;
    let s = `${ui(16, 22, "harbor / src / routes / ", "c-dim")}${ui(150, 22, "bag.tsx")}<circle cx="203" cy="18" r="3" fill="#ececec"/>
      <rect x="${ew - 64}" y="6" width="50" height="24" rx="6" fill="#2a2a2a"/>${ui(ew - 39, 22, "Done").replace("<text", `<text text-anchor="middle"`)}
      <rect y="36" width="${ew}" height="1" fill="#ffffff12"/>`;
    CODE.forEach(([n, src, mark], i) => {
      const y = top + i * ROW;
      if (mark === "sel") s += `<rect x="52" y="${y}" width="${ew - 52}" height="${ROW}" fill="${BG.sel}"/>`;
      if (mark === "edit") s += `<rect y="${y}" width="${ew}" height="${ROW}" fill="#ffffff0a"/>`;
      if (mark === "add" || mark === "edit") s += `<rect x="44" y="${y + 1}" width="2.5" height="${ROW - 2}" fill="${mark === "add" ? "#4eba65" : "#4d8ef7"}"/>`;
      s += `<text class="sv-mono c-dim" x="36" y="${y + 13}" text-anchor="end">${n}</text>`;
      s += term([src], { x: 60, y, w: ew });
    });
    // The quick edit: the caret after the words being typed.
    const editRow = CODE.findIndex(([, , m]) => m === "edit");
    s += `<rect class="caret" x="${60 + 44 * CH + 1}" y="${top + editRow * ROW + 2}" width="1.6" height="14" fill="#ececec"/>`;
    // The quote: the + on the line number, and the menu of agents it can go to.
    const qy = top + CODE.findIndex(([, , m]) => m === "sel") * ROW;
    s += `<rect x="40" y="${qy + 1}" width="16" height="16" rx="4" fill="${PIN}"/><path d="M48 ${qy + 5}v8M44 ${qy + 9}h8" stroke="#fff" stroke-width="1.6" stroke-linecap="round"/>`;
    const px = 200, py = qy + 2 * ROW + 8, pw = 250; // under the edited line, right of the short closing tags
    const item = (y, icon, name, note, on) => `${on ? `<rect x="${px + 6}" y="${y - 17}" width="${pw - 12}" height="28" rx="6" fill="#2c2c2c"/>` : ""}
      ${icon}${ui(px + 40, y + 1.5, name)}${note ? ui(px + pw - 16, y + 1.5, note, "c-dim").replace("<text", `<text text-anchor="end"`) : ""}`;
    s += `<g filter="url(#pop-shadow)"><rect x="${px}" y="${py}" width="${pw}" height="128" rx="10" fill="#1b1b1b" stroke="#ffffff1f"/></g>
      ${ui(px + 16, py + 24, "Quote bag.tsx:32 to", "c-dim")}
      ${item(py + 50, `<use href="#i-claude" x="${px + 16}" y="${py + 40}" width="14" height="14"/>`, "Claude Code", "wired", true)}
      ${item(py + 78, `<use href="#i-codex" x="${px + 16}" y="${py + 68}" width="14" height="14"/>`, "Codex", "")}
      ${item(py + 106, `<path d="M${px + 23} ${py + 97}v10M${px + 18} ${py + 102}h10" stroke="#cfcfcf" stroke-width="1.6" stroke-linecap="round"/>`, "New agent with this", "")}`;
    // The tree.
    s += `<rect x="${ew}" width="1" height="${h}" fill="#ffffff12"/>
      <rect x="${ew + 10}" y="8" width="${tw - 20}" height="26" rx="7" fill="#1c1c1c"/>${ui(ew + 20, 25, "Filter, or ? to search", "c-dim")}`;
    TREE.forEach(([depth, name, kind], i) => {
      const x = ew + 16 + depth * 14, y = 64 + i * 26;
      if (kind === "M") s += `<rect x="${ew + 6}" y="${y - 17}" width="${tw - 12}" height="24" rx="6" fill="#262626"/>${ui(ew + tw - 16, y, "M").replace("<text", `<text text-anchor="end" fill="#4d8ef7" style="fill:#4d8ef7"`)}`;
      const folder = kind === "open" || kind === "shut";
      if (folder) s += `<path d="${kind === "open" ? `M${x} ${y - 7}l4 4 4-4` : `M${x + 2} ${y - 9}l4 4-4 4`}" fill="none" stroke="#8d8d8d" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>`;
      s += ui(x + 14, y, name, folder || kind === "M" ? "" : "c-dim");
    });
    return s;
  };

  // --- the session ----------------------------------------------------------------------------------
  // Agents on one project, Harbor. In the hero Claude Code drives the web bag page and the iPhone while
  // a Codex on its own writes tests. In the shots Claude settles merge order with the Codex on the bag page, a
  // Codex runs checkout on the iPhone, and you fix a line of the bag page yourself.
  const T0 = [
    { t: "{dim|❯} make checkout match on the web and the iPhone", bg: "umsg" },
    "",
    "{@rec.green} {b|Read} 2 annotations on localhost:5173/bag",
    "  {@elbow.dim}  {dim|promo field: \"move it above the total\"}",
    "{@rec.green} {b|Update}(src/routes/bag.tsx)",
    "  {@elbow.dim}  {dim|Added} {b|18} {dim|lines, removed} {b|6} {dim|lines}",
    "{@rec.green} {b|Bash}(canopy pane read sim)",
    "  {@elbow.dim}  {dim|iPhone 17 Pro · Harbor 1.4 (dev)}",
    "{@rec} Tapping Place order on the iPhone.",
    "",
  ];
  const T2 = [
    { t: "{b|›} add tests for the promo code rules", bg: "umsg" },
    "",
    "{green|•} {b|Explored}",
    "  {dim|└ Read src/lib/promo.ts, promo.test.ts}",
    "{green|•} Expired and stacked codes are untested.",
    "  Adding cases for both, plus the 10% cap.",
    "{green|•} {b|Edited} src/lib/promo.test.ts {green|+42}",
    "{green|•} {b|Ran} npm test -- promo",
    "  {dim|└} {green|9 passed} {dim|(380ms)}",
    "",
    "{dim|•} {shimmer|Working} {dim|(18s • esc to interrupt)}",
  ];
  const T1 = [
    { t: "{dim|❯} ship the orders API, then agree", bg: "umsg" },
    { t: "  merge order with the web codex", bg: "umsg" },
    "",
    "{@rec.green} {b|Update}(src/api/orders.ts)",
    "  {@elbow.dim}  {dim|Added} {b|34} {dim|lines}",
    "{@rec.green} {b|Bash}(npm test -- orders)",
    "  {@elbow.dim}  {dim|12 passed}",
    "{@rec.green} {b|Bash}(hc send t4 \"#41 hits",
    "  cart.ts. yours?\")",
    "",
    { t: "{dim|<hc from=\"t4 codex\">}#42 renames", bg: "umsg" },
    { t: "  cart.total. Merge #41 first,", bg: "umsg" },
    { t: "  I'll rebase on top.{dim|</hc>}", bg: "umsg" },
    "",
    "{@rec} Merging {b|#41} now, then {b|#42}.",
  ];
  const T3 = [
    { t: "{b|›} test checkout on the iPhone and", bg: "umsg" },
    { t: "  fix what I marked", bg: "umsg" },
    "",
    "{green|•} {b|Read} 1 annotation on the iPhone",
    "  {dim|└ Express: \"show the date\"}",
    "{green|•} {b|Edited} ios/Delivery.swift {green|+3} {red|-1}",
    "{green|•} {b|Ran} xcodebuild -scheme Harbor",
    "  {dim|└ Build succeeded · 6.2s}",
    "{green|•} Express now reads Arrives Sat, Nov 28",
    "{green|•} Scrolling down to Payment",
    "{green|•} Pressing Place order",
    "  {dim|└ HB-2041 placed · $48.20}",
    "",
    "{dim|•} {shimmer|Working} {dim|(41s • esc to interrupt)}",
  ];
  const T4 = [
    { t: "{b|›} {dim|<hc from=\"t1 claude\">}#41 hits", bg: "umsg" },
    { t: "  cart.ts. yours?{dim|</hc>}", bg: "umsg" },
    "",
    "{green|•} {b|Ran} git diff --stat main",
    "  {dim|└ src/lib/cart.ts | 9} {green|+++++}{red|----}",
    "{green|•} #42 renames cart.total, so #41",
    "  should land first.",
    "{green|•} Replied to t1",
    "",
    "  {b|Would you like to run the}",
    "  {b|following command?}",
    "  {dim|$} git rebase main",
    "{cyan|›} {cyan|1. Yes, proceed}",
    "  2. No, tell Codex what to do",
  ];
  // The hero's panes, keyed by the ids in claude.html. `device` panes are a bare phone, no pane chrome.
  const HERO = {
    t1: { title: "Claude Code", sub: "checkout", icon: "claude", dot: "work", body: working(T0) },
    codex: { title: "Codex", sub: "promo tests", icon: "codex", dot: "work", body: codex(T2) },
    browser: { title: "Harbor · Your bag", icon: "globe", body: browser("localhost:5173/bag", harborWeb()) },
    sim: { device: harborCheckout() },
  };

  // --- the feature shots ------------------------------------------------------------------------------
  // Each is a crop of the canvas at 740 x 560, close to the size it is shown at, so the text reads at
  // about the app's own size.
  const W = 740, H = 560;
  const frame = (label, inner) => `<svg class="sv" viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(label)}">${inner}</svg>`;
  const placed = (...items) => items.map((p) => pane(p)).join("");

  const SHOTS = {
    wire: () => {
      const a = { x: 16, y: 16, w: 320, h: 420 }, b = { x: 420, y: 124, w: 304, h: 420 };
      return frame("Claude Code and Codex wired together, agreeing which pull request merges first",
        wire(a, b, { talk: true }) + placed(
          { ...a, title: "Claude Code", sub: "orders api", icon: "claude", dot: "work", body: claude(T1) },
          { ...b, title: "Codex", sub: "bag page", icon: "codex", dot: "wait", body: codex(T4, { waiting: true }) }));
    },
    sim: () => {
      const a = { x: 16, y: 104, w: 344, h: 440 }, b = { x: 470, y: 16, w: 252, h: 528 };
      return frame("Codex placing an order in the Harbor iPhone app, after fixing the row marked on its screen",
        wire(a, b) + placed({ ...a, title: "Codex", sub: "checkout on iPhone", icon: "codex", dot: "work", body: codex(T3) }) +
        `<svg x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}">${phone(harborCheckout({ annot: true }))(b.w, b.h)}</svg>`);
    },
    files: () => frame("The Files panel: bag.tsx open beside its file tree, one line quoted to Claude Code and another being edited",
      placed({ x: 16, y: 16, w: 708, h: 528, title: "Files", sub: "harbor", icon: "folder", dot: null, body: files })),
  };

  // --- the phone's loop: scroll down, press Place order, show the confirmation, start again ----------
  function playCheckout(svg) {
    const scroll = svg.querySelector("[data-scroll]"), btn = svg.querySelector("[data-btn]"), done = svg.querySelector("[data-done]");
    const tl = gsap.timeline({ repeat: -1, repeatDelay: .6, paused: true })
      .set(scroll, { y: 0 })
      .to(scroll, { y: -230, duration: 1.8, ease: "power2.inOut" }, 1.2)
      .to(btn, { scale: .96, svgOrigin: "201 803", duration: .14, ease: "power1.out" }, "+=.9")
      .to(btn, { scale: 1, duration: .3, ease: "power2.out" })
      .to(done, { opacity: 1, duration: .4, ease: "power1.out" }, "-=.15")
      .to({}, { duration: 2.4 })
      .to(done, { opacity: 0, duration: .35 })
      .set(scroll, { y: 0 }, "<");
    ScrollTrigger.create({ trigger: svg, onToggle: (s) => (s.isActive ? tl.play() : tl.pause()) });
  }

  // Claude Code's spinner, as on the Ignsight site: the glyph breathes every 120ms, the verb changes, the
  // timer counts. It ticks only while its pane is on screen.
  const GLYPHS = ["·", "✢", "✳", "✶", "✻", "✽", "✻", "✶", "✳", "✢"], VERBS = ["Percolating", "Cogitating", "Noodling", "Discombobulating"];
  function startWorking(svg) {
    const spin = svg.querySelector("[data-spin]"), verb = svg.querySelector("[data-verb]"), secs = svg.querySelector("[data-secs]");
    let frame = 0, timer;
    const tick = () => {
      frame++;
      spin.textContent = GLYPHS[frame % GLYPHS.length];
      if (frame % 8 === 0) secs.textContent = 12 + frame / 8 | 0;
      if (frame % 28 === 0) verb.textContent = VERBS[(frame / 28) % VERBS.length] + "…";
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
  for (const el of document.querySelectorAll("[data-scene]")) el.innerHTML = SHOTS[el.dataset.scene]();
  gsap.registerPlugin(ScrollTrigger);
  if (matchMedia("(prefers-reduced-motion: no-preference)").matches) {
    document.querySelectorAll("[data-checkout]").forEach(playCheckout);
    document.querySelectorAll("[data-working]").forEach(startWorking);
  } else document.querySelectorAll("[data-talk]").forEach((g) => g.remove());

  return { wirePath };
})();

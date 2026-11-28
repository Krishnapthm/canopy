// The three feature shots on index.html, drawn from Scenes' parts:
// - wire: Claude Code and Codex are each at work on their own task; Claude Code messages the Codex it is
//   wired to, and Codex replies. A message shows in full only in the agent that receives it.
// - sim: you ask Codex to test checkout on the iPhone; it taps through, you mark the delivery rows, it
//   fixes them.
// - files: the docked Files panel. Open bag.tsx, checkout.tsx or index.tsx from the tree; hover the code
//   to move the quote button, or drag across lines to select a range, click the button to quote them to an
//   agent, with a note if you like. Each file has Git changes in the tree and in the gutter.
// The two stories are GSAP timelines, built fresh for every run so each starts from a clean slate. They
// play only while their shot is on screen. Needs gsap and scenes.js.

(() => {
  const { esc, CH, ROW, BG, PIN, term, pane, wire, tx, harborCheckout, phone } = Scenes;
  const NS = "http://www.w3.org/2000/svg";
  const W = 740, H = 560;
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const frame = (label, inner) => `<svg class="sv" viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(label)}">${inner}</svg>`;

  // --- typing --------------------------------------------------------------------------------------
  // Word-wraps plain text to `cols` columns, the way a TUI wraps its prompt.
  function wrap(text, cols) {
    const lines = [""];
    for (const word of text.split(" ")) {
      const last = lines.length - 1;
      if (!lines[last]) lines[last] = word;
      else if (lines[last].length + 1 + word.length <= cols) lines[last] += " " + word;
      else lines.push(word);
    }
    return lines;
  }
  // When each keystroke lands, as [characters typed, seconds]: a person types in bursts of one or two
  // characters, and pauses after a comma or a full stop.
  function keystrokes(text) {
    const out = [];
    let t = 0;
    for (let i = 1; i <= text.length; i += 1 + Math.floor(Math.random() * 2)) {
      out.push([i, t]);
      t += .03 + Math.random() * .045 + (/[,.]/.test(text[i - 1]) ? .2 : 0);
    }
    out.push([text.length, t]);
    return { out, end: t };
  }

  // --- live terminals ------------------------------------------------------------------------------
  const GLYPHS = ["·", "✢", "✳", "✶", "✻", "✽", "✻", "✶", "✳", "✢"];
  const VERBS = ["Brewing", "Noodling", "Pondering"]; // short enough for a narrow pane

  // A terminal pane body (for Scenes.pane) whose transcript is written as the story plays: an empty feed
  // above the agent's composer. `kind` is "claude" or "codex".
  const liveTerm = (kind) => (w, h) => {
    const spin = kind === "claude"
      ? `<text class="c-clawd" x="16" y="13" data-glyph>✻</text><text x="39" y="13" xml:space="preserve"><tspan class="c-clawd c-shimmer" data-verb>Percolating…</tspan><tspan class="c-dim"> (<tspan data-secs>0</tspan>s · esc to interrupt)</tspan></text>`
      : `<text x="16" y="13" xml:space="preserve"><tspan class="c-dim">•</tspan> <tspan class="c-shimmer">Working</tspan><tspan class="c-dim"> (<tspan data-secs>0</tspan>s • esc to interrupt)</tspan></text>`;
    return `<g data-term="${kind}" data-w="${w}" data-h="${h}">
      <svg data-view width="${w}" height="${h}" overflow="hidden"><g data-feed><g class="sv-mono" data-spin opacity="0">${spin}</g></g></svg>
      <g data-composer></g></g>`;
  };

  // Drives one live terminal. Each beat schedules itself on a timeline at time `t` (seconds) and returns
  // the time the next beat can start, so a story reads top to bottom.
  function terminal(g) {
    const kind = g.dataset.term, w = +g.dataset.w, h = +g.dataset.h;
    const view = g.querySelector("[data-view]"), feed = g.querySelector("[data-feed]");
    const spin = g.querySelector("[data-spin]"), composer = g.querySelector("[data-composer]");
    const glyph = spin.querySelector("[data-glyph]"), verb = spin.querySelector("[data-verb]"), secs = spin.querySelector("[data-secs]");
    const box = g.closest(".sv-pane"), dot = box.querySelector("[data-dot]");
    const width = Math.floor((w - 32) / CH), cols = width - 2; // a row's columns, and the prompt's after "❯ "
    let rows = 0, spinning = false, top = h;

    // Keeps the newest row in view, the way a terminal follows its output.
    function follow() {
      const bottom = 10 + (rows + (spinning ? 1 : 0)) * ROW + 6;
      gsap.to(feed, { y: Math.min(0, top - 4 - bottom), duration: calm ? 0 : .3, ease: "power2.out", overwrite: true });
    }
    // The composer: Claude Code's prompt between two rules over its mode line, or Codex's shaded prompt
    // over its model line. It grows a row for each wrapped line of input.
    function compose(text = "", typing = false) {
      const lines = wrap(text, cols);
      let list, caretRow;
      if (kind === "claude") {
        const rule = `{rule|${"─".repeat(width)}}`;
        list = [rule, ...lines.map((l, i) => `${i ? "  " : "{hi|❯} "}${l}`), rule, "  {@play.violet}{@play.violet}{violet| accept edits on} {dim|· Opus 5.5}"];
        caretRow = lines.length;
      } else {
        const pad = { t: "", bg: "umsg" }; // the shaded box has a row of padding above and below
        list = [pad, ...lines.map((l, i) => ({ t: i ? `  ${l}` : `{b|›} ${l || "{dim|Ask Codex to do anything}"}`, bg: "umsg" })), pad, "", "  {dim|gpt-6.2-sol medium · ~/harbor}"];
        caretRow = lines.length;
      }
      top = h - 8 - list.length * ROW;
      const caret = typing || kind === "claude"
        ? `<rect class="caret" x="${16 + (2 + [...lines.at(-1)].length) * CH}" y="${top + caretRow * ROW + 1}" width="${CH}" height="16" fill="#ececec" fill-opacity=".85"/>` : "";
      composer.innerHTML = term(list, { y: top, w }) + caret;
      view.setAttribute("height", Math.max(0, top - 4));
      follow();
    }
    function print(r) {
      const el = feed.appendChild(document.createElementNS(NS, "g"));
      el.setAttribute("transform", `translate(0 ${10 + rows * ROW})`);
      el.innerHTML = term([r], { x: 16, y: 0, w });
      rows++;
      spin.setAttribute("transform", `translate(0 ${10 + rows * ROW})`);
      follow();
    }
    function spinner(on) {
      spinning = on;
      spin.setAttribute("transform", `translate(0 ${10 + rows * ROW})`);
      gsap.set(spin, { opacity: on ? 1 : 0 });
      follow();
    }
    const state = (cls) => dot.setAttribute("class", `dot ${cls}`);

    return {
      feed,
      outline: box.querySelector("[data-outline]"),
      reset() {
        feed.replaceChildren(spin);
        rows = 0;
        spinner(false);
        gsap.set(feed, { y: 0 });
        compose();
        state("idle");
      },
      // You type `text` and press return; it lands in the transcript as your message.
      type(tl, t, text) {
        const { out, end } = keystrokes(text);
        for (const [n, at] of out) tl.call(compose, [text.slice(0, n), true], t + .45 + at);
        const mark = kind === "claude" ? "{dim|❯}" : "{b|›}";
        return this.enter(tl, t + .45 + end + .4, wrap(text, cols).map((l, i) => ({ t: `${i ? "  " : mark + " "}${l}`, bg: "umsg" })));
      },
      // Canopy delivers a message: it is pasted into the prompt whole, then sent. `shown` is how the
      // transcript prints it.
      paste(tl, t, text, shown) {
        tl.call(compose, [text, true], t);
        return this.enter(tl, t + .55, shown);
      },
      enter(tl, t, shown) {
        tl.call(() => { compose(); shown.forEach(print); state("work"); }, null, t);
        return t + .35;
      },
      say(tl, t, r, pause = .42) {
        tl.call(print, [r], t);
        return t + pause;
      },
      // The agent's working line for `s` seconds: Claude Code's breathing glyph and verb, or Codex's
      // Working, each with its timer.
      busy(tl, t, s) {
        const p = { f: 0 }, word = VERBS[Math.floor(Math.random() * VERBS.length)];
        let last = -1;
        tl.call(() => { if (verb) verb.textContent = word + "…"; spinner(true); }, null, t);
        tl.to(p, { f: s / .12, duration: s, ease: "none", onUpdate: () => {
          const f = Math.floor(p.f);
          if (f === last) return;
          last = f;
          if (glyph) glyph.textContent = GLYPHS[f % GLYPHS.length];
          secs.textContent = Math.floor(f * .12);
        } }, t);
        tl.call(spinner, [false], t + s);
        return t + s;
      },
      state(tl, t, cls) {
        tl.call(state, [cls], t);
        return t;
      },
    };
  }

  // A message rides the wire `path` (backwards when `back`) and the pane it reaches lights up.
  function carry(tl, t, msg, path, back, outline) {
    const len = path.getTotalLength(), p = { k: 0 };
    tl.set(msg, { opacity: 1 }, t);
    tl.to(p, { k: 1, duration: .9, ease: "power1.inOut", onUpdate: () => {
      const pt = path.getPointAtLength((back ? 1 - p.k : p.k) * len);
      msg.setAttribute("cx", pt.x);
      msg.setAttribute("cy", pt.y);
    } }, t);
    tl.set(msg, { opacity: 0 }, t + .9);
    tl.to(outline, { attr: { "stroke-opacity": .6 }, duration: .15, ease: "power1.out" }, t + .8);
    tl.to(outline, { attr: { "stroke-opacity": .11 }, duration: .8, ease: "power2.out" }, t + .95);
    return t + .9;
  }

  // Plays a story on its shot: `reset()` clears the stage, then `build(tl)` writes one run into a fresh
  // timeline, which starts the next run when it ends. With reduced motion the shot shows the run's last
  // frame instead. Mid-run fromTo tweens pass immediateRender: false, or they would show their start
  // state the moment the run is built.
  function play(shot, reset, build) {
    let tl, seen = false;
    if (calm) {
      reset();
      tl = gsap.timeline({ paused: true });
      build(tl);
      tl.progress(1);
      return;
    }
    const run = () => {
      tl?.kill();
      reset();
      tl = gsap.timeline({ paused: !seen, onComplete: run });
      build(tl);
    };
    run();
    new IntersectionObserver(([e]) => { seen = e.isIntersecting; tl.paused(!seen); }).observe(shot);
  }
  // A run fades `targets` in, and at its end holds the last frame, then fades them out so the next run
  // starts clean. Reduced motion keeps the last frame.
  const close = (tl, t, targets) => { if (!calm) tl.to(targets, { opacity: 0, duration: .4, ease: "power1.in" }, t + 3.4); };
  const open = (tl, targets) => tl.fromTo(targets, { opacity: 0 }, { opacity: 1, duration: calm ? 0 : .4, ease: "power1.out" }, 0);

  // --- wire: Claude Code and Codex agree a merge order ------------------------------------------------
  function wireShot(el) {
    const a = { x: 16, y: 16, w: 320, h: 420 }, b = { x: 420, y: 124, w: 304, h: 420 };
    el.innerHTML = frame("Claude Code and Codex wired together, agreeing which pull request merges first",
      wire(a, b) +
      pane({ ...a, title: "Claude Code", sub: "orders api", icon: "claude", body: liveTerm("claude") }) +
      pane({ ...b, title: "Codex", sub: "bag page", icon: "codex", body: liveTerm("codex") }) +
      `<circle class="wire-msg" data-msg r="3.5" opacity="0"/>`);
    const svg = el.firstElementChild, path = svg.querySelector(".wire"), msg = svg.querySelector("[data-msg]");
    const [claude, codex] = [...svg.querySelectorAll("[data-term]")].map(terminal);
    const feeds = [claude.feed, codex.feed];

    // Both agents start on their own work. The wire carries each message to the other agent only: the
    // sender's transcript says it is sending, the receiver's shows the message itself.
    play(el, () => { claude.reset(); codex.reset(); gsap.set(msg, { opacity: 0 }); }, (tl) => {
      open(tl, feeds);
      let c = claude.type(tl, .6, "ship the orders api, then agree merge order with the web codex");
      let x = codex.type(tl, 1.1, "add the promo field to the bag page");
      c = claude.say(tl, c, "", 0);
      c = claude.busy(tl, c, 1.4);
      c = claude.say(tl, c, "{@rec.green} {b|Update}(src/api/orders.ts)", .3);
      c = claude.say(tl, c, "  {@elbow.dim}  {dim|Added} {b|34} {dim|lines}", .7);
      c = claude.say(tl, c, "{@rec.green} {b|Bash}(npm test -- orders)", .9);
      c = claude.say(tl, c, "  {@elbow.dim}  {dim|12 passed}", .6);
      x = codex.say(tl, x, "", 0);
      x = codex.busy(tl, x, 1.2);
      x = codex.say(tl, x, "{green|•} {b|Edited} bag.tsx {green|+18} {red|-6}", .8);
      x = codex.say(tl, x, "{green|•} {b|Ran} npm test -- bag", .8);
      x = codex.say(tl, x, "  {dim|└} {green|7 passed}", .6);

      c = claude.say(tl, c, "{@rec} Sending t4 a message about", 0);
      c = claude.say(tl, c, "  which PR merges first.", .3);
      let t = carry(tl, Math.max(c, x), msg, path, false, codex.outline);

      t = codex.paste(tl, t, "<hc from=\"t1 claude\">#41 hits cart.ts. yours?</hc>", [
        { t: "{dim|<hc from=\"t1 claude\">}#41 hits", bg: "umsg" },
        { t: "  cart.ts. yours?{dim|</hc>}", bg: "umsg" },
      ]);
      t = codex.say(tl, t, "", 0);
      t = codex.busy(tl, t, 1.3);
      t = codex.say(tl, t, "{green|•} {b|Ran} git diff --stat main", .4);
      t = codex.say(tl, t, "  {dim|└ src/lib/cart.ts | 9} {green|+++++}{red|----}", .7);
      t = codex.say(tl, t, "{green|•} #42 renames cart.total, so #41", 0);
      t = codex.say(tl, t, "  should land first.", .6);
      t = codex.say(tl, t, "{green|•} Sending t1 a message about the", 0);
      t = codex.say(tl, t, "  merge order.", .3);
      t = codex.state(tl, t, "done");
      t = carry(tl, t, msg, path, true, claude.outline);

      t = claude.say(tl, t, "", 0);
      t = claude.paste(tl, t, "<hc from=\"t4 codex\">#42 renames cart.total. Merge #41 first, I'll rebase on top.</hc>", [
        { t: "{dim|<hc from=\"t4 codex\">}#42 renames", bg: "umsg" },
        { t: "  cart.total. Merge #41 first,", bg: "umsg" },
        { t: "  I'll rebase on top.{dim|</hc>}", bg: "umsg" },
      ]);
      t = claude.say(tl, t, "", 0);
      t = claude.busy(tl, t, 1);
      t = claude.say(tl, t, "{@rec} Merging {b|#41} now, then {b|#42}.", .2);
      t = claude.state(tl, t, "done");
      close(tl, t, feeds);
    });
  }

  // --- sim: Codex tests checkout on the iPhone, you mark a fix, Codex makes it ----------------------
  // The app starts with each delivery row's radio before its price; you ask for the price first.
  function simShot(el) {
    const a = { x: 16, y: 16, w: 400, h: 528 }, b = { x: 470, y: 16, w: 252, h: 528 };
    el.innerHTML = frame("Codex taps through checkout in the Harbor iPhone app, then moves the delivery prices you marked",
      wire(a, b) +
      pane({ ...a, title: "Codex", sub: "checkout on iPhone", icon: "codex", body: liveTerm("codex") }) +
      `<svg x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}">${phone(harborCheckout(), { checkout: false })(b.w, b.h)}</svg>` +
      `<circle class="wire-msg" data-msg r="3.5" opacity="0"/>`);
    const svg = el.firstElementChild, path = svg.querySelector(".wire"), msg = svg.querySelector("[data-msg]");
    const codex = terminal(svg.querySelector("[data-term]"));
    const screen = svg.querySelector("[data-screen]");

    // Drawn over the screen, in points: a simulator touch, canopy's oval mark with its pin and note, and
    // your pointer.
    const NOTE = "price first, then the radio";
    screen.insertAdjacentHTML("beforeend", `
      <circle data-touch r="14" fill="#000" fill-opacity=".16" stroke="#000" stroke-opacity=".22" stroke-width="2" opacity="0"/>
      <g data-mark opacity="0">
        <path data-pen d="M318 346A68 58 0 1 1 318 462A68 58 0 1 1 318 346Z" fill="none" stroke="${PIN}" stroke-width="4"/>
        <g data-pin><circle cx="318" cy="346" r="15" fill="${PIN}"/>${tx(318, 351.5, "1", { size: 15, weight: 650, fill: "#fff", anchor: "middle" })}</g>
        <g data-note opacity="0">
          <rect x="14" y="286" width="288" height="46" rx="12" fill="#1b1b1b" stroke="#fff" stroke-opacity=".14"/>
          <text class="sv-ui" data-note-text x="30" y="314" style="font-size:15px"></text>
          <rect class="caret" data-note-caret x="30" y="299" width="1.6" height="20" fill="#ececec"/>
          <rect data-send x="244" y="295" width="50" height="28" rx="8" fill="${PIN}" fill-opacity="1"/>
          <text class="sv-ui" x="269" y="314" text-anchor="middle" style="font-size:14px">Send</text>
        </g>
      </g>
      <path data-cursor d="M0 0v30l8-8 5.5 12 4.5-2-5.5-12H24z" fill="#000" stroke="#fff" stroke-width="2" stroke-linejoin="round" opacity="0"/>`);
    const q = (sel) => screen.querySelector(sel);
    const scroll = q("[data-scroll]"), touch = q("[data-touch]"), cursor = q("[data-cursor]");
    const mark = q("[data-mark]"), pen = q("[data-pen]"), pin = q("[data-pin]"), note = q("[data-note]");
    const noteText = q("[data-note-text]"), noteCaret = q("[data-note-caret]"), send = q("[data-send]");
    const radios = Object.fromEntries([...screen.querySelectorAll("[data-radio]")].map((r) => [r.dataset.radio, r.querySelector("[data-on]")]));
    const moved = [q('[data-radio="standard"]'), q('[data-radio="express"]')];
    const prices = [...screen.querySelectorAll("[data-price]")];
    const shipping = q("[data-shipping]"), total = q("[data-total]"), button = q("[data-btn] text");
    const penLength = pen.getTotalLength();
    pen.setAttribute("stroke-dasharray", penLength);
    const app = [scroll, ...moved, ...prices]; // what the app draws, faded between runs

    const choose = (tl, t, on, off) => tl.to(radios[on], { opacity: 1, duration: .16 }, t).to(radios[off], { opacity: 0, duration: .16 }, t);
    const tap = (tl, t, x, y) => {
      tl.set(touch, { attr: { cx: x, cy: y } }, t);
      tl.fromTo(touch, { opacity: 1, attr: { r: 14 } }, { opacity: 0, attr: { r: 26 }, duration: .5, ease: "power2.out", immediateRender: false }, t);
      return t + .2;
    };
    const writeNote = (s) => {
      noteText.textContent = s;
      noteCaret.setAttribute("x", 31 + noteText.getComputedTextLength());
    };

    play(el, () => {
      codex.reset();
      gsap.set(scroll, { y: 0 });
      for (const [name, on] of Object.entries(radios)) gsap.set(on, { opacity: name === "standard" || name === "applepay" ? 1 : 0 });
      gsap.set(moved, { x: -56 }); // the radio sits before the price, the layout you will mark
      gsap.set(prices, { x: 38 });
      shipping.textContent = "$4.90"; total.textContent = "$48.20"; button.textContent = "Place order · $48.20";
      gsap.set([mark, note, cursor, touch, msg], { opacity: 0 });
      gsap.set(pin, { scale: 0, transformOrigin: "50% 50%" });
      pen.setAttribute("stroke-dashoffset", penLength);
      writeNote("");
    }, (tl) => {
      open(tl, [codex.feed, ...app]);
      let t = codex.type(tl, .6, "test checkout on the iphone, tap through it");
      t = codex.say(tl, t, "", 0);
      t = codex.busy(tl, t, 1.2);
      t = codex.say(tl, t, "{green|•} {b|Ran} canopy pane read sim", .3);
      t = codex.say(tl, t, "  {dim|└ iPhone 17 Pro · Harbor 1.4 (dev)}", .8);

      // Express, then the new totals.
      t = tap(tl, t, 300, 436);
      choose(tl, t, "express", "standard");
      tl.call(() => { shipping.textContent = "$12.00"; total.textContent = "$55.30"; button.textContent = "Place order · $55.30"; }, null, t);
      t = codex.say(tl, t + .4, "{green|•} {b|Tapped} Express under Delivery", 0);
      t = codex.say(tl, t, "  {dim|└ Shipping $12.00 · total $55.30}", .5);

      // Down to Payment and Visa, then back up.
      tl.to(scroll, { y: -230, duration: 1, ease: "power2.inOut" }, t);
      t = tap(tl, t + 1.2, 356, 372);
      choose(tl, t, "visa", "applepay");
      t = codex.say(tl, t + .4, "{green|•} {b|Scrolled} to Payment and tapped Visa", .5);
      tl.to(scroll, { y: 0, duration: .9, ease: "power2.inOut" }, t);
      t = codex.say(tl, t + 1, "{green|•} Checkout works. Each choice updates", 0);
      t = codex.say(tl, t, "  the total.", .2);
      t = codex.state(tl, t, "done") + .7;

      // You box the delivery prices, pin a note and send it.
      const start = pen.getPointAtLength(0);
      tl.set(mark, { opacity: 1 }, t);
      tl.fromTo(cursor, { opacity: 0, x: 210, y: 640 }, { opacity: 1, duration: .25, immediateRender: false }, t);
      tl.to(cursor, { x: start.x, y: start.y, duration: .7, ease: "power2.inOut" }, t + .1);
      t += .9;
      tl.to(pen, { attr: { "stroke-dashoffset": 0 }, duration: 1.2, ease: "power1.inOut", onUpdate: () => {
        const pt = pen.getPointAtLength(penLength - +pen.getAttribute("stroke-dashoffset"));
        gsap.set(cursor, { x: pt.x, y: pt.y });
      } }, t);
      t += 1.25;
      tl.to(pin, { scale: 1, duration: .35, ease: "back.out(2.2)" }, t);
      tl.fromTo(note, { opacity: 0, y: 6 }, { opacity: 1, y: 0, duration: .25, ease: "power2.out", immediateRender: false }, t + .15);
      t += .6;
      const { out, end } = keystrokes(NOTE);
      for (const [n, at] of out) tl.call(writeNote, [NOTE.slice(0, n)], t + at);
      t += end + .35;
      tl.to(cursor, { x: 266, y: 312, duration: .5, ease: "power2.inOut" }, t);
      t += .55;
      tl.to(send, { attr: { "fill-opacity": .7 }, duration: .08 }, t).to(send, { attr: { "fill-opacity": 1 }, duration: .2 }, t + .08);
      tl.to(note, { opacity: 0, duration: .25 }, t + .15);
      tl.to(cursor, { opacity: 0, duration: .3 }, t + .3);
      t = carry(tl, t + .3, msg, path, true, codex.outline);

      // Codex reads the mark and fixes the rows; the app reloads with the price first.
      t = codex.say(tl, t, "", 0);
      t = codex.paste(tl, t, "[Image #1] 1 annotation on the iPhone", [{ t: "{b|›} {dim|[Image #1]} 1 annotation on the iPhone", bg: "umsg" }]);
      t = codex.say(tl, t, "", 0);
      t = codex.busy(tl, t, 1.3);
      t = codex.say(tl, t, "{green|•} {b|Read} 1 annotation on the iPhone", 0);
      t = codex.say(tl, t, `  {dim|└ Delivery: "${NOTE}"}`, .7);
      t = codex.say(tl, t, "{green|•} {b|Edited} ios/DeliveryRow.swift {green|+2} {red|-2}", .8);
      t = codex.say(tl, t, "{green|•} {b|Ran} xcodebuild -scheme Harbor", 1.1);
      t = codex.say(tl, t, "  {dim|└ Build succeeded · 6.2s}", .3);
      tl.to(mark, { opacity: 0, duration: .3 }, t);
      tl.to([...moved, ...prices], { x: 0, duration: .7, ease: "power3.inOut" }, t + .2);
      t = codex.say(tl, t + 1.1, "{green|•} Delivery rows now show the price, then", 0);
      t = codex.say(tl, t, "  the radio.", .2);
      t = codex.state(tl, t, "done");
      close(tl, t, [codex.feed, ...app]);
    });
  }

  // --- files: the docked Files panel ------------------------------------------------------------------
  // Each file's lines from `from`, in the row markup of Scenes.term, with an optional Git mark: "add"
  // (green bar) or "edit" (blue bar, the line you are editing). `rest` is the line number the quote
  // button waits on before you hover.
  const FILES = {
    "bag.tsx": { from: 24, rest: 32, lines: [
      ["{kw|export function} {fn|Summary}({ bag }: { bag: {type|Bag} }) {"],
      ["  {kw|const} total = {fn|useTotal}(bag);"],
      ["  {kw|return} ("],
      ["    <{fn|Card} {type|title}={amber|\"Order summary\"}>"],
      ["      <{fn|Row} {type|label}={amber|\"Subtotal\"} {type|value}={bag.subtotal} />"],
      ["      <{fn|Row} {type|label}={amber|\"Shipping\"} {type|value}={bag.shipping} />"],
      ["      <{fn|PromoField} {type|onApply}={bag.applyCode} />", "add"],
      ["      <{fn|Divider} />", "add"],
      ["      <{fn|Row} {type|label}={amber|\"Total\"} {type|value}={total} {type|strong} />"],
      ["      <{fn|Button} {type|onClick}={checkout}>Place order", "edit"],
      ["      </{fn|Button}>"],
      ["    </{fn|Card}>"],
      ["  );"],
      ["}"],
    ] },
    "checkout.tsx": { from: 1, rest: 13, lines: [
      ["{kw|import} { Button, Card, Radio, Row } {kw|from} {amber|\"../ui\"};"],
      ["{kw|import} { placeOrder, useBag } {kw|from} {amber|\"../lib/bag\"};"],
      ["{kw|import} { useState } {kw|from} {amber|\"react\"};"],
      [""],
      ["{kw|const} DELIVERY = ["],
      ["  { id: {amber|\"standard\"}, label: {amber|\"Standard\"}, price: {amber|4.9} },"],
      ["  { id: {amber|\"express\"}, label: {amber|\"Express\"}, price: {amber|12} },"],
      ["];"],
      [""],
      ["{kw|export function} {fn|Checkout}() {"],
      ["  {kw|const} bag = {fn|useBag}();"],
      ["  {kw|const} [pick, setPick] = {fn|useState}({amber|\"standard\"});", "add"],
      ["  {kw|const} ship = DELIVERY.{fn|find}((d) => d.id === pick);", "add"],
      [""],
      ["  {kw|return} ("],
      ["    <{fn|Card} {type|title}={amber|\"Delivery\"}>"],
      ["      {DELIVERY.{fn|map}((d) => ("],
      ["        <{fn|Row} {type|key}={d.id} {type|label}={d.label} {type|value}={d.price}"],
      ["          {type|end}={<{fn|Radio} {type|checked}={d.id === pick} />}"],
      ["          {type|onPress}={() => {fn|setPick}(d.id)} />"],
      ["      ))}"],
      ["      <{fn|Button} {type|label}={amber|\"Place order\"}"],
      ["        {type|onClick}={() => {fn|placeOrder}(bag, ship)} />", "edit"],
      ["    </{fn|Card}>"],
      ["  );"],
      ["}"],
    ] },
    "index.tsx": { from: 1, rest: 12, lines: [
      ["{kw|import} { Link } {kw|from} {amber|\"react-router\"};"],
      ["{kw|import} { Grid, ProductCard } {kw|from} {amber|\"../ui\"};"],
      ["{kw|import} { coffeeUrl, useProducts } {kw|from} {amber|\"../lib/products\"};"],
      [""],
      ["{kw|export function} {fn|Index}() {"],
      ["  {kw|const} products = {fn|useProducts}();"],
      [""],
      ["  {kw|return} ("],
      ["    <{fn|main}>"],
      ["      <{fn|h1}>Fresh from the roaster</{fn|h1}>", "add"],
      ["      <{fn|p}>Small batches, shipped Monday from Portland.</{fn|p}>", "add"],
      ["      <{fn|Grid}>"],
      ["        {products.{fn|map}((p) => ("],
      ["          <{fn|Link} {type|key}={p.id} {type|to}={{fn|coffeeUrl}(p)}>"],
      ["            <{fn|ProductCard}", "edit"],
      ["              {type|name}={p.name}"],
      ["              {type|note}={p.roast}"],
      ["              {type|price}={p.price}"],
      ["            />"],
      ["          </{fn|Link}>"],
      ["        ))}"],
      ["      </{fn|Grid}>"],
      ["    </{fn|main}>"],
      ["  );"],
      ["}"],
    ] },
  };
  // The tree: depth, name, and "open"/"shut" for folders or "M" for a changed file.
  const TREE = [
    [0, "harbor", "open"], [1, "ios", "shut"], [1, "src", "open"], [2, "api", "shut"], [2, "lib", "shut"], [2, "routes", "open"],
    [3, "bag.tsx", "M"], [3, "checkout.tsx", "M"], [3, "index.tsx", "M"], [1, ".env"], [1, ".gitignore"], [1, "package.json"], [1, "README.md"],
  ];
  const AGENTS = [["#i-claude", "Claude Code", "wired"], ["#i-codex", "Codex", ""], ["#i-cursor", "Cursor", ""], ["#i-gemini", "Gemini CLI", ""], [null, "New agent with this", ""]];

  function filesShot(el) {
    const pw = 708, ph = 528, tw = 216, ew = pw - tw; // panel, tree and editor widths
    const STRIP = 36, TOP = 42, ROW_H = 28;          // tab strip height, first code row, tree row height
    const ui = (x, y, s, cls = "", attrs = "") => `<text class="sv-ui ${cls}" x="${x}" y="${y}"${attrs}>${esc(s)}</text>`;
    const fileIcon = (x, y) => `<path d="M${x + 3} ${y + 1.5}h5.5l3.5 3.5v8.5h-9z" fill="none" stroke="#8d8d8d" stroke-width="1.2" stroke-linejoin="round"/>`;
    const folderIcon = (x, y) => `<use class="sv-ic" href="#i-folder" x="${x}" y="${y}" width="15" height="15" style="stroke:#8d8d8d;stroke-width:1.6"/>`;

    const tree = TREE.map(([depth, name, kind], i) => {
      const y = STRIP + 52 + i * ROW_H, x = ew + 14 + depth * 18, folder = kind === "open" || kind === "shut";
      const chevron = folder ? `<path d="${kind === "open" ? `M${x} ${y + 12}l4 4 4-4` : `M${x + 2} ${y + 10}l4 4-4 4`}" fill="none" stroke="#8d8d8d" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>` : "";
      const hit = FILES[name] ? `<rect class="tree-hit" data-open="${name}" x="${ew + 6}" y="${y}" width="${tw - 12}" height="${ROW_H}" rx="7"/>` : "";
      return `${chevron}${folder ? folderIcon(x + 15, y + 6.5) : fileIcon(x + 15, y + 6.5)}
        ${ui(x + 38, y + 18, name, folder || FILES[name] ? "" : "c-dim")}
        ${kind === "M" ? ui(ew + tw - 16, y + 18, "M", "", ` text-anchor="end" style="fill:#4d8ef7"`) : ""}${hit}`;
    }).join("");

    el.innerHTML = frame("The Files panel, docked: open bag.tsx, checkout.tsx or index.tsx from the tree, and hover a line to quote it to an agent",
      `<svg data-dock x="16" y="16" width="${pw}" height="${ph}" viewBox="0 0 ${pw} ${ph}">
        <clipPath id="dock-clip"><rect width="${pw}" height="${ph}" rx="10"/></clipPath>
        <g clip-path="url(#dock-clip)"><rect width="${pw}" height="${ph}" fill="#0c0c0c"/>
          <g data-tabs></g>
          <!-- Expand panel and Close panel, as Lucide draws them (maximize-2, x). -->
          <svg class="sv-ic" x="${pw - 66}" y="11" width="14" height="14" viewBox="0 0 24 24" style="stroke:#8d8d8d;stroke-width:2"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>
          <svg class="sv-ic" x="${pw - 34}" y="11" width="14" height="14" viewBox="0 0 24 24" style="stroke:#8d8d8d;stroke-width:2"><path d="M18 6 6 18M6 6l12 12"/></svg>
          <rect y="${STRIP}" width="${pw}" height="1" fill="#ffffff12"/>
          <g data-editor>
            <rect x="0" y="${STRIP + 1}" width="${ew}" height="${ph - STRIP}" fill="transparent"/>
            <rect data-hl x="52" y="0" width="${ew - 52}" height="${ROW}" fill="${BG.sel}" opacity="0"/>
            <g data-code></g>
            <g data-plus opacity="0"><rect x="40" y="1" width="16" height="16" rx="4" fill="${PIN}"/><path d="M48 5v8M44 9h8" stroke="#fff" stroke-width="1.6" stroke-linecap="round"/></g>
          </g>
          <rect x="${ew}" y="${STRIP}" width="1" height="${ph - STRIP}" fill="#ffffff12"/>
          <rect x="${ew + 10}" y="${STRIP + 10}" width="${tw - 20}" height="30" rx="8" fill="none" stroke="#fff" stroke-opacity=".11"/>
          <circle cx="${ew + 27}" cy="${STRIP + 24.5}" r="4.5" fill="none" stroke="#8d8d8d" stroke-width="1.3"/><path d="M${ew + 30.5} ${STRIP + 28}l3 3" stroke="#8d8d8d" stroke-width="1.3" stroke-linecap="round"/>
          ${ui(ew + 40, STRIP + 29, "Filter, or ? to search", "c-dim")}
          <rect data-picked x="${ew + 6}" y="0" width="${tw - 12}" height="${ROW_H}" rx="7" fill="#262626"/>
          ${tree}
          <g data-pop opacity="0" visibility="hidden"></g>
        </g>
        <rect x=".5" y=".5" width="${pw - 1}" height="${ph - 1}" rx="10" fill="none" stroke="#fff" stroke-opacity=".11"/>
      </svg>`);
    const dock = el.querySelector("[data-dock]");
    const $ = (sel) => dock.querySelector(sel);
    const tabs = $("[data-tabs]"), editor = $("[data-editor]"), code = $("[data-code]"), hl = $("[data-hl]");
    const plus = $("[data-plus]"), pop = $("[data-pop]"), picked = $("[data-picked]");
    const glide = { duration: .18, ease: "power3.out" };
    const moveHl = gsap.quickTo(hl, "y", glide), movePlus = gsap.quickTo(plus, "y", glide);
    const movePicked = gsap.quickTo(picked, "y", glide);
    const rowY = (i) => TOP + i * ROW;
    const plain = (src) => [...src.replace(/\{[\w ]+\|([^}]*)\}/g, "$1")]; // a row's characters, without its markup

    let tabbed = ["bag.tsx"], file = "bag.tsx", quoting = false;
    let range = null, anchor = null; // the selected lines [first, last] as indices into the file, and where a drag began
    let comment = "", pick = 0;      // the quote menu: the note being typed, and the agent highlighted

    // The tab strip: one tab per open file, the active one raised, each with its close button.
    function drawTabs() {
      tabs.innerHTML = tabbed.map((name) => `<g data-tab="${name}" class="dock-tab${name === file ? " on" : ""}">
        <rect class="tab-bg" y="6" height="24" rx="7"/>${fileIcon(10, 10.5)}${ui(30, 22, name, name === file ? "" : "c-dim")}
        <g data-close="${name}" class="tab-close"><rect y="10" width="16" height="16" rx="4"/><path d="M4 14l8 8M12 14l-8 8" transform="translate(0 0)"/></g></g>`).join("");
      let x = 8;
      for (const tab of tabs.children) {
        const w = 30 + tab.querySelector("text").getComputedTextLength() + 34;
        tab.setAttribute("transform", `translate(${x} 0)`);
        tab.querySelector(".tab-bg").setAttribute("width", w);
        tab.querySelector(".tab-close").setAttribute("transform", `translate(${w - 22} 0)`);
        x += w + 4;
      }
    }
    // The editor: the file's lines with their numbers and Git bars, or a hint when no file is open.
    function drawCode() {
      const f = FILES[file];
      if (!f) {
        code.innerHTML = ui(ew / 2, ph / 2, "Open a file from the tree", "c-dim", ` text-anchor="middle"`);
        return;
      }
      code.innerHTML = f.lines.map(([src, mark], i) => {
        const y = rowY(i);
        return `${mark === "edit" ? `<rect y="${y}" width="${ew}" height="${ROW}" fill="#ffffff0a"/>` : ""}
          ${mark ? `<rect x="44" y="${y + 1}" width="2.5" height="${ROW - 2}" fill="${mark === "add" ? "#4eba65" : "#4d8ef7"}"/>` : ""}
          <text class="sv-mono c-dim" x="36" y="${y + 13}" text-anchor="end">${f.from + i}</text>${term([src], { x: 60, y, w: ew })}
          ${mark === "edit" ? `<rect class="caret" x="${60 + plain(src).length * CH + 1}" y="${y + 2}" width="1.6" height="14" fill="#ececec"/>` : ""}`;
      }).join("");
    }
    // Puts the quote button and its highlight on line `i` (an index into the file), or hides them.
    function hover(i, instant = false) {
      range = i === null ? null : [i, i];
      gsap.to([hl, plus], { opacity: range ? 1 : 0, duration: .15, overwrite: "auto" });
      hl.setAttribute("height", ROW);
      if (!range) return;
      if (instant) gsap.set([hl, plus], { y: rowY(i) });
      else { moveHl(rowY(i)); movePlus(rowY(i)); }
    }
    // Selects lines `a` to `b`, as when you drag across them: the highlight covers them all and the
    // quote button sits on the first.
    function select(a, b) {
      range = [Math.min(a, b), Math.max(a, b)];
      hl.setAttribute("height", (range[1] - range[0] + 1) * ROW);
      gsap.killTweensOf([hl, plus]);
      gsap.set([hl, plus], { y: rowY(range[0]), opacity: 1 });
    }
    const multi = () => range && range[0] !== range[1];
    const rest = () => (FILES[file]?.rest ? FILES[file].rest - FILES[file].from : null);

    function show(name) {
      closeQuote();
      if (name && !tabbed.includes(name)) tabbed.push(name);
      file = name;
      drawTabs();
      drawCode();
      gsap.fromTo(code, { opacity: 0 }, { opacity: 1, duration: .18, ease: "power1.out" });
      const row = TREE.findIndex(([, n]) => n === name);
      gsap.to(picked, { opacity: row < 0 ? 0 : 1, duration: .15 });
      if (row >= 0) movePicked(STRIP + 52 + row * ROW_H);
      hover(rest(), true);
    }
    function shut(name) {
      const i = tabbed.indexOf(name);
      tabbed.splice(i, 1);
      show(name === file ? tabbed[Math.min(i, tabbed.length - 1)] ?? null : file);
    }

    // The quote menu under the selected lines: a required note, then send to Claude Code (wired to it),
    // Codex, or a new agent. Typing writes the note; return sends to the highlighted agent.
    const POP_X = 200;
    function openQuote() {
      const f = FILES[file], [a, b] = range, w = 250, h = 82 + AGENTS.length * 28, px = POP_X;
      const lines = a === b ? `${f.from + a}` : `${f.from + a}-${f.from + b}`;
      let py = rowY(b) + ROW + 6;
      if (py + h > ph - 8) py = Math.max(STRIP + 4, rowY(a) - h - 6);
      comment = ""; pick = 0;
      pop.innerHTML = `<g filter="url(#pop-shadow)"><rect x="${px}" y="${py}" width="${w}" height="${h}" rx="10" fill="#1b1b1b" stroke="#ffffff1f"/></g>
        ${ui(px + 16, py + 24, `Quote ${file}:${lines} to`, "c-dim")}
        <rect x="${px + 12}" y="${py + 34}" width="${w - 24}" height="28" rx="7" fill="#111" stroke="#fff" stroke-opacity=".14"/>
        ${ui(px + 22, py + 53, "", "", " data-comment")}<rect class="caret" data-comment-caret x="${px + 22}" y="${py + 40}" width="1.6" height="16" fill="#ececec"/>
        <rect data-item-hl x="${px + 6}" y="${py + 71}" width="${w - 12}" height="28" rx="6" fill="#2c2c2c"/>
        ${AGENTS.map(([icon, name, note], i) => {
          const y = py + 88 + i * 28;
          return `<g data-item="${i}"><rect x="${px + 6}" y="${y - 17}" width="${w - 12}" height="28" fill="transparent"/>
            ${icon ? `<use href="${icon}" x="${px + 16}" y="${y - 10}" width="14" height="14"/>` : `<path d="M${px + 23} ${y - 9}v10M${px + 18} ${y - 4}h10" stroke="#cfcfcf" stroke-width="1.6" stroke-linecap="round"/>`}
            ${ui(px + 40, y + 1.5, name)}${note ? ui(px + w - 16, y + 1.5, note, "c-dim", ` text-anchor="end"`) : ""}</g>`;
        }).join("")}`;
      drawComment();
      quoting = true;
      gsap.fromTo(pop, { autoAlpha: 0, y: -4 }, { autoAlpha: 1, y: 0, duration: .16, ease: "power2.out" });
    }
    // The note field: what you typed, or its hint, with the caret after it.
    function drawComment() {
      const text = pop.querySelector("[data-comment]");
      if (!text) return;
      text.textContent = comment || "describe it or ask to explain";
      text.classList.toggle("c-dim", !comment);
      pop.querySelector("[data-comment-caret]").setAttribute("x", POP_X + 22 + (comment ? text.getComputedTextLength() + 1 : 0));
    }
    function closeQuote() {
      if (!quoting) return;
      quoting = false;
      gsap.to(pop, { autoAlpha: 0, duration: .12 });
    }
    // Sends the quote: the menu closes and the selection flashes. The note is required, so an empty one stays open.
    function sendQuote() {
      if (!comment.trim()) return;
      closeQuote();
      gsap.fromTo(hl, { fill: "#3b4d78" }, { fill: BG.sel, duration: .6, ease: "power2.out" });
    }

    // Pointer: hovering the code moves the quote button line by line, dragging selects lines; the tree
    // and tabs open files.
    const local = (e) => new DOMPoint(e.clientX, e.clientY).matrixTransform(dock.getScreenCTM().inverse());
    const lineAt = (e) => Math.floor((local(e).y - TOP) / ROW);
    const last = () => FILES[file].lines.length - 1;
    editor.addEventListener("pointerdown", (e) => {
      if (quoting || !FILES[file] || e.target.closest("[data-plus]")) return;
      const i = lineAt(e);
      if (i < 0 || i > last()) return;
      anchor = i;
      select(i, i);
      editor.setPointerCapture(e.pointerId);
    });
    editor.addEventListener("pointermove", (e) => {
      if (quoting || !FILES[file]) return;
      const i = lineAt(e);
      if (anchor !== null) select(anchor, Math.min(Math.max(i, 0), last()));
      else if (!multi() && i >= 0 && i <= last() && i !== range?.[0]) hover(i);
    });
    editor.addEventListener("pointerup", () => { anchor = null; });
    editor.addEventListener("pointerleave", () => { if (!quoting && anchor === null && !multi()) hover(rest()); });
    pop.addEventListener("pointermove", (e) => {
      const item = e.target.closest("[data-item]");
      if (!item) return;
      pick = +item.dataset.item;
      gsap.to(pop.querySelector("[data-item-hl]"), { y: pick * 28, duration: .14, ease: "power3.out" });
    });
    dock.addEventListener("click", (e) => {
      const target = e.target.closest("[data-close], [data-tab], [data-open], [data-plus], [data-item]");
      if (!target) return closeQuote();
      const { close: closing, tab, open: name, item } = target.dataset;
      if (closing) shut(closing);
      else if (tab) show(tab);
      else if (name) show(name);
      else if (item !== undefined) sendQuote();
      else if (quoting) closeQuote();
      else if (range) openQuote();
    });
    // With the menu open, typing writes the note and return sends; escape closes the menu, then the selection.
    addEventListener("keydown", (e) => {
      if (e.key === "Escape") return quoting ? closeQuote() : multi() && hover(rest());
      if (!quoting) return;
      if (e.key === "Enter") sendQuote();
      else if (e.key === "Backspace") comment = comment.slice(0, -1);
      else if (e.key.length === 1 && !e.metaKey && !e.ctrlKey && comment.length < 28) comment += e.key;
      else return;
      e.preventDefault();
      drawComment();
    });

    show("bag.tsx");
    document.fonts.ready.then(drawTabs); // tab widths are measured in Geist, which may still be loading
  }

  wireShot(document.querySelector('[data-scene="wire"]'));
  simShot(document.querySelector('[data-scene="sim"]'));
  filesShot(document.querySelector('[data-scene="files"]'));
})();

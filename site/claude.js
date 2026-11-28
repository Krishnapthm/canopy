// Landing page behaviour: smooth scrolling and in-page links, the footer light that follows the
// pointer, and the install copy button. Needs gsap and lenis.

const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;

// --- scrolling --------------------------------------------------------------------------------------
// Lenis eases wheel scrolling on GSAP's frame clock, so scrolling and animation share one frame. Links
// to a part of this page (#features, #install, #top) glide there instead of navigating. Reduced motion
// keeps the browser's own scrolling and jumps.
const lenis = calm ? null : new Lenis({ allowNestedScroll: true });
if (lenis) {
  gsap.ticker.add((time) => lenis.raf(time * 1000));
  gsap.ticker.lagSmoothing(0);
}
document.addEventListener("click", (e) => {
  const link = e.target.closest?.('a[href^="#"]');
  if (!link || e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
  const target = link.hash === "#top" ? 0 : document.getElementById(decodeURIComponent(link.hash.slice(1)));
  if (target === null) return;
  e.preventDefault();
  history.pushState(null, "", link.hash);
  if (lenis) lenis.scrollTo(target, { duration: 1.2, easing: (t) => 1 - (1 - t) ** 4 });
  else if (target === 0) scrollTo(0, 0);
  else target.scrollIntoView();
});

// --- footer light ------------------------------------------------------------------------------------
// The embossed icon and wordmark are lit by one distant light that turns to come from the pointer, and
// eases back to the top left when it leaves. Only the direction moves; a light that came down onto the
// sheet would wash the flat plastic out.
//
// WebGL draws the light with the math of the SVG filter #footer-emboss in index.html: a blurred height
// map of the artwork, Sobel normals, diffuse and specular lighting, faded out past the bevel and laid
// over the cobalt with hard-light. The height map is built once per size, so turning the light is one
// small shader pass on the GPU instead of the browser re-running the SVG filter, on the CPU in Safari
// and WebKit, every frame. Without WebGL the SVG filter stays and its lights turn instead.
const footer = document.querySelector("#footer");
const brand = footer.querySelector(".footer-brand");
const light = { azimuth: 225 }; // degrees, clockwise from +x on screen; 225 is the top left
const lamp = footerLight();
const lights = footer.querySelectorAll("[data-light]");
const paint = () => {
  if (lamp?.ready()) lamp.draw(light.azimuth);
  else for (const l of lights) l.setAttribute("azimuth", light.azimuth.toFixed(1));
};
// Turn the short way round: pick the equivalent angle nearest the current one.
const near = (deg) => deg + 360 * Math.round((light.azimuth - deg) / 360);
const turnTo = (deg, duration) => gsap.to(light, { azimuth: near(deg), duration, ease: "power2.out", overwrite: true, onUpdate: paint });
if (!calm) {
  footer.addEventListener("pointermove", (e) => {
    if (e.pointerType !== "mouse") return;
    const box = brand.getBoundingClientRect(); // aim from the artwork's centre
    turnTo(Math.atan2(e.clientY - (box.top + box.height / 2), e.clientX - (box.left + box.width / 2)) * 180 / Math.PI, 1.8);
  });
  footer.addEventListener("pointerleave", () => turnTo(225, 2.4));
}

function footerLight() {
  // The filter's numbers, from #footer-emboss.
  const BLUR = 6.5, SCALE = 12.48, ELEVATION = 35, DIFFUSE = .872, SPECULAR = .14, SHINE = 10, FALLOFF = 12;
  const canvas = document.createElement("canvas");
  canvas.className = "footer-gl";
  canvas.setAttribute("aria-hidden", "true");
  const gl = canvas.getContext("webgl", { premultipliedAlpha: true, antialias: false, depth: false });
  if (!gl) return null;

  const program = gl.createProgram();
  for (const [type, src] of [[gl.VERTEX_SHADER, `
    attribute vec2 p; varying vec2 v;
    void main() { v = vec2(p.x * .5 + .5, .5 - p.y * .5); gl_Position = vec4(p, 0., 1.); }`],
  // Height is packed in two bytes (r high, g low) for 16 bits of precision; 8 would band the slopes.
  // Normals use the filter's Sobel kernel one device pixel apart, as the browser computes it.
  [gl.FRAGMENT_SHADER, `
    precision highp float;
    uniform sampler2D map; uniform vec2 px; uniform vec3 L; uniform vec3 H; varying vec2 v;
    float h(float x, float y) { vec4 c = texture2D(map, v + vec2(x, y) * px); return (c.r * 65280. + c.g * 255.) / 65535.; }
    void main() {
      float tl = h(-1., -1.), t = h(0., -1.), tr = h(1., -1.), l = h(-1., 0.), c = h(0., 0.), r = h(1., 0.), bl = h(-1., 1.), b = h(0., 1.), br = h(1., 1.);
      vec3 n = normalize(vec3(-${SCALE} * .25 * ((tr + 2. * r + br) - (tl + 2. * l + bl)),
                              -${SCALE} * .25 * ((bl + 2. * b + br) - (tl + 2. * t + tr)), 1.));
      float lit = min(${DIFFUSE} * max(dot(n, L), 0.) + ${SPECULAR} * pow(max(dot(n, H), 0.), ${SHINE}.), 1.);
      float a = min(1., ${FALLOFF}. * c);
      gl_FragColor = vec4(vec3(lit) * a, a);
    }`]]) {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    gl.attachShader(program, s);
  }
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) return null;
  gl.useProgram(program);
  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
  const corner = gl.getAttribLocation(program, "p");
  gl.enableVertexAttribArray(corner);
  gl.vertexAttribPointer(corner, 2, gl.FLOAT, false, 0, 0);
  gl.bindTexture(gl.TEXTURE_2D, gl.createTexture());
  for (const [key, value] of [[gl.TEXTURE_MIN_FILTER, gl.NEAREST], [gl.TEXTURE_MAG_FILTER, gl.NEAREST], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) {
    gl.texParameteri(gl.TEXTURE_2D, key, value);
  }
  const uPx = gl.getUniformLocation(program, "px"), uL = gl.getUniformLocation(program, "L"), uH = gl.getUniformLocation(program, "H");

  let ready = false;
  // The height map: the artwork's coverage, drawn where the page lays it out, then blurred as
  // feGaussianBlur does. Rebuilt when the footer changes size.
  async function build() {
    await document.fonts.ready;
    const dpr = Math.min(devicePixelRatio, 2);
    const box = brand.getBoundingClientRect(), home = footer.getBoundingClientRect();
    // The filter region: 116% x 150% of the artwork's box, from -8% and -25%.
    const x = box.left - box.width * .08, y = box.top - box.height * .25, rw = box.width * 1.16, rh = box.height * 1.5;
    const w = Math.round(rw * dpr), h = Math.round(rh * dpr);
    if (!w || !h) return;
    Object.assign(canvas.style, { left: `${x - home.left}px`, top: `${y - home.top}px`, width: `${rw}px`, height: `${rh}px` });
    canvas.width = w;
    canvas.height = h;

    const art = document.createElement("canvas");
    art.width = w;
    art.height = h;
    const ctx = art.getContext("2d", { willReadFrequently: true });
    ctx.setTransform(dpr, 0, 0, dpr, -x * dpr, -y * dpr); // draw in viewport coordinates
    drawArtwork(ctx);
    const pixels = ctx.getImageData(0, 0, w, h).data;
    const height = new Float32Array(w * h);
    for (let i = 0; i < height.length; i++) height[i] = pixels[i * 4 + 3] / 255;
    gaussian(height, w, h, BLUR * dpr);
    const packed = new Uint8Array(w * h * 4);
    for (let i = 0; i < height.length; i++) {
      const q = Math.round(Math.min(1, height[i]) * 65535);
      packed[i * 4] = q >> 8;
      packed[i * 4 + 1] = q & 255;
    }
    gl.viewport(0, 0, w, h);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, packed);
    gl.uniform2f(uPx, 1 / w, 1 / h);
    ready = true;
    draw(light.azimuth);
    if (!canvas.isConnected) {
      footer.querySelector(".footer-relief").after(canvas);
      footer.classList.add("gl");
    }
  }
  // The icon and the wordmark, filled solid at their places on screen.
  function drawArtwork(ctx) {
    ctx.fillStyle = "#fff";
    // The icon: the #mark symbol's paths, fitted into its svg box as viewBox does (meet, centred).
    const icon = brand.querySelector("svg").getBoundingClientRect();
    const k = Math.min(icon.width / 411, icon.height / 416);
    ctx.save();
    ctx.translate(icon.left + (icon.width - 411 * k) / 2, icon.top + (icon.height - 416 * k) / 2);
    ctx.scale(k, k);
    ctx.translate(-32, -51);
    for (const p of document.querySelectorAll("#mark path")) ctx.fill(new Path2D(p.getAttribute("d")));
    ctx.restore();
    // The wordmark, glyph by glyph where the page set each one, on the page's baseline.
    const span = brand.querySelector("span"), text = span.firstChild, style = getComputedStyle(span);
    ctx.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
    const probe = span.appendChild(document.createElement("span"));
    probe.style.cssText = "display:inline-block;width:0;height:0;vertical-align:baseline";
    const baseline = probe.getBoundingClientRect().top;
    probe.remove();
    const range = document.createRange();
    for (let i = 0; i < text.length; i++) {
      range.setStart(text, i);
      range.setEnd(text, i + 1);
      ctx.fillText(text.data[i], range.getBoundingClientRect().left, baseline);
    }
  }
  function draw(azimuth) {
    const a = azimuth * Math.PI / 180, e = ELEVATION * Math.PI / 180;
    const L = [Math.cos(a) * Math.cos(e), Math.sin(a) * Math.cos(e), Math.sin(e)];
    const H = [L[0], L[1], L[2] + 1], n = Math.hypot(...H);
    gl.uniform3f(uL, ...L);
    gl.uniform3f(uH, H[0] / n, H[1] / n, H[2] / n);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  canvas.addEventListener("webglcontextlost", () => { ready = false; footer.classList.remove("gl"); canvas.remove(); });
  let pending = 0;
  // Rebuild once resizing settles: the blur is CPU work, too much to redo every frame of a drag.
  new ResizeObserver(() => { clearTimeout(pending); pending = setTimeout(build, pending ? 150 : 0); }).observe(footer);
  return { draw, ready: () => ready };
}

// feGaussianBlur's own approximation, three box blurs of width d (SVG spec), on `data` in place.
function gaussian(data, w, h, sigma) {
  const d = Math.floor(sigma * 3 * Math.sqrt(2 * Math.PI) / 4 + .5);
  const tmp = new Float32Array(data.length);
  // Box-blurs `lines` runs of `len` samples, `step` apart within a run and `stride` apart between runs.
  // Each output averages `size` samples starting `offset` before it; samples past the edge are 0.
  const pass = (src, dst, len, lines, step, stride, size, offset) => {
    for (let line = 0; line < lines; line++) {
      const base = line * stride, at = (i) => (i < 0 || i >= len ? 0 : src[base + i * step]);
      let sum = 0;
      for (let i = -offset; i < size - offset; i++) sum += at(i);
      for (let i = 0; i < len; i++) {
        dst[base + i * step] = sum / size;
        sum += at(i + size - offset) - at(i - offset);
      }
    }
  };
  // Odd d: three centred boxes. Even d: two boxes centred half a pixel left then right, then d + 1.
  const boxes = d % 2 ? [[d, d >> 1], [d, d >> 1], [d, d >> 1]] : [[d, d / 2], [d, d / 2 - 1], [d + 1, d / 2]];
  for (const [size, offset] of boxes) {
    pass(data, tmp, w, h, 1, w, size, offset);
    pass(tmp, data, h, w, w, 1, size, offset);
  }
}

// --- install ------------------------------------------------------------------------------------------
// Copy button: shown only where the clipboard API exists; falls back to selecting the command.
const copyButton = document.querySelector(".copy-button");
const command = document.querySelector("#install-command");
const copyStatus = document.querySelector("#copy-status");
if (navigator.clipboard?.writeText) {
  let reset;
  copyButton.hidden = false;
  copyButton.addEventListener("click", async () => {
    clearTimeout(reset);
    try {
      await navigator.clipboard.writeText(command.textContent);
      copyButton.textContent = "copied";
      copyStatus.textContent = "install command copied";
    } catch {
      getSelection().selectAllChildren(command);
      copyButton.textContent = "selected";
      copyStatus.textContent = "install command selected - copy it with your keyboard";
    }
    reset = setTimeout(() => { copyButton.textContent = "copy"; copyStatus.textContent = ""; }, 2000);
  });
}

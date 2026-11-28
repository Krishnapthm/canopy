// Landing page behaviour: the footer light that follows the pointer, and the install
// copy button.

const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;

// Footer light: the emboss filter's light turns to come from the pointer, the way the injection-moulded
// study lights its bevels, and eases back to the top left when the pointer leaves. Only the direction
// moves; a light that came down onto the sheet would wash the flat plastic out.
const footer = document.querySelector("#footer");
const lights = footer.querySelectorAll("[data-light]");
const brand = footer.querySelector(".footer-brand");
const light = { azimuth: 225 };
const paint = () => {
  const angle = light.azimuth.toFixed(1);
  for (const lamp of lights) {
    if (lamp.getAttribute("azimuth") !== angle) lamp.setAttribute("azimuth", angle);
  }
};
// Turn the short way round: pick the equivalent angle nearest the current one.
const near = (deg) => deg + 360 * Math.round((light.azimuth - deg) / 360);
const turnTo = (deg, duration) => {
  const from = light.azimuth;
  const to = near(deg);
  if (Math.abs(to - from) < 0.05) { stopAnimation(light); return; }
  animate(light, duration * 1000, (progress) => {
    light.azimuth = from + (to - from) * progress;
    paint();
  });
};
if (!calm) {
  footer.addEventListener("pointermove", (e) => {
    if (e.pointerType !== "mouse") return;
    const box = brand.getBoundingClientRect();
    turnTo(Math.atan2(e.clientY - (box.top + box.height / 2), e.clientX - (box.left + box.width / 2)) * 180 / Math.PI, .26);
  });
  footer.addEventListener("pointerleave", () => turnTo(225, .65));
}

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

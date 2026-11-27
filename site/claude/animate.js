// Keep one animation frame pending per moving object.
const animationFrames = new WeakMap();

function stopAnimation(target) {
  cancelAnimationFrame(animationFrames.get(target));
  animationFrames.delete(target);
}

function animate(target, duration, draw) {
  stopAnimation(target);
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
    draw(1);
    return;
  }

  const start = performance.now();
  function frame(now) {
    const progress = Math.min(1, Math.max(0, (now - start) / duration));
    draw(1 - (1 - progress) ** 4);
    if (progress < 1) animationFrames.set(target, requestAnimationFrame(frame));
    else animationFrames.delete(target);
  }
  animationFrames.set(target, requestAnimationFrame(frame));
}

// Canopy's page script for browser panes (docs/wiki/Browser-Pane.md), injected at document
// start after the bundled react-grab, whose own start is turned off. Pick starts react-grab
// with telemetry off; the picked element goes to Canopy as {pick: {...}} over wry's IPC.
// Agent-facing data only: no source or HTML leaves the page beyond what Canopy cuts.
(() => {
  if (window.__canopy) return;
  const post = (message) => window.ipc.postMessage(JSON.stringify(message));

  // A CSS path from the nearest element with an id, at most five steps.
  const selector = (el) => {
    const steps = [];
    for (let node = el; node && node.nodeType === 1 && steps.length < 5; node = node.parentElement) {
      if (node.id) { steps.unshift(`#${CSS.escape(node.id)}`); break; }
      let step = node.localName;
      const parent = node.parentElement;
      if (parent) {
        const same = [...parent.children].filter((c) => c.localName === node.localName);
        if (same.length > 1) step += `:nth-of-type(${same.indexOf(node) + 1})`;
      }
      steps.unshift(step);
    }
    return steps.join(" > ");
  };

  // The element's opening tag; Canopy cuts it to 80 characters.
  const openTag = (el) => {
    const attrs = [...el.attributes].map((a) => (a.value ? ` ${a.name}="${a.value}"` : ` ${a.name}`)).join("");
    return `<${el.localName}${attrs.slice(0, 200)}>`;
  };

  let api = null;
  async function picked(el) {
    let source = null;
    try { source = await api.getSource(el); } catch {}
    api.deactivate();
    const text = (el.innerText ?? el.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 400);
    post({ pick: {
      url: location.href,
      component: source?.componentName ?? null,
      file: source?.filePath ?? null,
      line: source?.lineNumber ?? null,
      column: source?.columnNumber ?? null,
      tag: openTag(el),
      selector: selector(el),
      text,
    } });
  }

  window.__canopy = {
    pick() {
      try {
        if (!api) {
          api = globalThis.__REACT_GRAB_MODULE__.init({ telemetry: false });
          // Returning true keeps react-grab from copying the element to the clipboard.
          api.registerPlugin({ name: "canopy", hooks: { onElementSelect: (el) => { picked(el); return true; } } });
        }
        api.activate();
      } catch (error) {
        post({ error: `The picker did not start: ${error}` });
      }
    },
  };
})();

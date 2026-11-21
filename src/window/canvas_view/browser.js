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
  function grab() {
    if (!api) {
      api = globalThis.__REACT_GRAB_MODULE__.init({ telemetry: false });
      // Returning true keeps react-grab from copying the element to the clipboard.
      api.registerPlugin({ name: "canopy", hooks: { onElementSelect: (el) => { picked(el); return true; } } });
    }
    return api;
  }

  async function source(el) {
    try {
      const where = await grab().getSource(el);
      if (where) return { component: where.componentName, file: where.filePath, line: where.lineNumber, column: where.columnNumber };
    } catch {}
    return {};
  }

  const cut = (value, limit) => [...value.slice(0, limit * 2)].slice(0, limit).join("");
  const text = (el, limit) => cut((el.innerText ?? el.textContent ?? "").replace(/\s+/g, " ").trim(), limit);
  const rect = ({ x, y, width, height }) => [x, y, width, height];
  const caret = (x, y) => document.caretRangeFromPoint(x, y);
  const styleNames = ["display", "position", "width", "height", "margin", "padding", "font-size", "font-weight", "color", "background-color"];
  const useful = "img, svg, picture, video, canvas, a, button, input, select, textarea, label, [role=button]";

  async function picked(el) {
    const where = await source(el);
    api.deactivate();
    post({ pick: { url: location.href, ...where, tag: openTag(el), selector: selector(el), text: text(el, 400) } });
  }

  window.__canopy = {
    pick() {
      try {
        grab().activate();
      } catch (error) {
        post({ error: `The picker did not start: ${error}` });
      }
    },
    async at(x, y) {
      let el = document.elementFromPoint(x, y);
      if (!el) return null;
      el = el.closest("svg") ?? el;
      while (el.parentElement && el.matches("div, span") && !el.matches(useful)
        && el.childElementCount === 1
        && ![...el.childNodes].some(n => n.nodeType === Node.TEXT_NODE && n.textContent.trim())) {
        el = el.parentElement;
      }
      const style = getComputedStyle(el);
      return {
        selector: selector(el), tag: cut(openTag(el), 80), text: text(el, 120),
        html: cut(el.outerHTML, 300), rect: rect(el.getBoundingClientRect()),
        styles: Object.fromEntries(styleNames.map(name => [name, style.getPropertyValue(name)])),
        ...await source(el),
      };
    },
    isText(x, y) {
      const range = caret(x, y);
      if (!range || range.startContainer.nodeType !== Node.TEXT_NODE) return false;
      range.selectNodeContents(range.startContainer);
      return [...range.getClientRects()].some(r => x >= r.left && x < r.right && y >= r.top && y < r.bottom);
    },
    textAt(x1, y1, x2, y2) {
      const a = caret(x1, y1);
      const b = caret(x2, y2);
      if (!a || !b) return null;
      const [start, end] = a.compareBoundaryPoints(Range.START_TO_START, b) <= 0 ? [a, b] : [b, a];
      const range = document.createRange();
      range.setStart(start.startContainer, start.startOffset);
      range.setEnd(end.startContainer, end.startOffset);
      const common = range.commonAncestorContainer;
      const el = common.nodeType === Node.ELEMENT_NODE ? common : common.parentElement;
      const selected = cut(range.toString(), 300);
      return selected && el ? { text: selected, selector: selector(el), rects: [...range.getClientRects()].map(rect) } : null;
    },
    rects(selectors) {
      return selectors.map(path => {
        try {
          const el = document.querySelector(path);
          return el ? rect(el.getBoundingClientRect()) : null;
        } catch { return null; }
      });
    },
    size: () => [innerWidth, innerHeight],
  };
})();

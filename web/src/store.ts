import { createStore } from "solid-js/store";
import type { Node, Item, Event } from "./protocol.gen";
export const [state, set] = createStore({
  nodes: {} as Record<string, Node>,
  sessions: {} as Record<string, { items: Item[] }>,
  selected: "", view: "tree", filter: "", palette: false, newAgent: false,
  child: false, connected: false, error: "", delivery: "", harnesses: [] as string[],
  collapsed: {} as Record<string, boolean>,
});
export function items(session: string, incoming: Item[], replace = false) {
  const previous = replace ? [] : state.sessions[session]?.items ?? [];
  const merged = new Map(previous.map(i => [i.id, i]));
  for (const item of incoming) merged.set(item.id, item);
  set("sessions", session, { items: [...merged.values()].sort((a,b) => a.seq-b.seq).slice(-300) });
}
export function ingest(event: Event) {
  if (event.t === "node") set("nodes", event.node.id, event.node);
  else if (event.t === "item") items(event.item.session, [event.item]);
  else {
    const index = state.sessions[event.session]?.items.findIndex(i => i.id === event.item) ?? -1;
    if (index >= 0) set("sessions", event.session, "items", index, "body", body => ({
      ...(typeof body === "object" && !Array.isArray(body) ? body : {}),
      text: String((body as Record<string, unknown>)?.text ?? "") + event.text,
    }));
  }
}
export function outline() {
  const rows: { node: Node; depth: number }[] = [];
  const nodes = Object.values(state.nodes);
  function walk(parent: string | null, depth: number) {
    for (const node of nodes.filter(n => n.parent === parent).sort((a,b) => a.created-b.created)) {
      if (!state.filter || (node.title+" "+node.harness+" "+node.id).toLowerCase().includes(state.filter.toLowerCase()))
        rows.push({node, depth});
      if (!state.collapsed[node.id]) walk(node.id, depth+1);
    }
  }
  walk(null, 0);
  return rows;
}
export function text(item: Item) {
  return typeof item.body === "object" && item.body !== null && !Array.isArray(item.body)
    ? String(item.body.text ?? JSON.stringify(item.body)) : String(item.body ?? "");
}

import { useSyncExternalStore } from "react";
import type { OutsideSession, Node, Item, Event } from "./protocol.gen";
export let state = {
  outside: [] as OutsideSession[],
  discovering: false, outsideSelected: "",
  nodes: {} as Record<string, Node>,
  sessions: {} as Record<string, { items: Item[] }>,
  selected: "", view: "tree", filter: "", palette: false, newAgent: false,
  child: false, connected: false, delivery: "", harnesses: [] as string[],
  collapsed: {} as Record<string, boolean>,
};
const listeners = new Set<() => void>();
export const listen = (fn: () => void) => { listeners.add(fn); return () => { listeners.delete(fn); }; };
export function useStore() { return useSyncExternalStore(listen, () => state); }
let batching = false;
export function batch(fn: () => void) {
  batching = true;
  try { fn(); } finally { batching = false; listeners.forEach(fn => fn()); }
}
export function set<K extends keyof typeof state>(key: K, value: (typeof state)[K]): void;
export function set(key: "collapsed", id: string, value: boolean): void;
export function set(key: keyof typeof state, value: unknown, nested?: boolean) {
  state = { ...state, ...(key === "selected" && value ? {outsideSelected:""} : {}), [key]: nested === undefined ? value : { ...state.collapsed, [String(value)]: nested } };
  if (!batching) listeners.forEach(fn => fn());
}
export function items(session: string, incoming: Item[], replace = false) {
  const previous = replace ? [] : state.sessions[session]?.items ?? [];
  const merged = new Map(previous.map(i => [i.id, i]));
  for (const item of incoming) merged.set(item.id, item);
  set("sessions", { ...state.sessions, [session]: { items: [...merged.values()].sort((a,b) => a.seq-b.seq).slice(-300) } });
}
/** True when a tree event moves a node into failed. Only events count: the tree loaded on connect is history. */
export function newlyFailed(node: Node) {
  return node.status.state === "failed" && state.nodes[node.id]?.status.state !== "failed";
}
export function ingest(event: Event) {
  if (event.t === "node") set("nodes", { ...state.nodes, [event.node.id]: event.node });
  else if (event.t === "item") items(event.item.session, [event.item]);
  else {
    const index = state.sessions[event.session]?.items.findIndex(i => i.id === event.item) ?? -1;
    if (index >= 0) {
      const item = state.sessions[event.session].items[index];
      const body = item.body;
      items(event.session, [{ ...item, body: {
        ...(typeof body === "object" && body !== null && !Array.isArray(body) ? body : {}),
        text: String((body as Record<string, unknown>)?.text ?? "") + event.text,
      }}]);
    }
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
export function permissionSummary(item: Item) {
  const body = item.body;
  if (typeof body !== "object" || body === null || Array.isArray(body)) return {title:text(item), summary:""};
  const tool = body.tool;
  const input = body.input ?? (typeof tool === "object" && tool !== null && !Array.isArray(tool) ? tool.rawInput : undefined) ?? body.request;
  const summary = typeof input === "string" ? input : typeof input === "object" && input !== null && !Array.isArray(input)
    ? Object.entries(input).filter(([,value]) => typeof value === "string" || typeof value === "number" || typeof value === "boolean").map(([key,value]) => `${key}: ${value}`).join("; ") : "";
  return {
    title: typeof body.title === "string" ? body.title : typeof tool === "string" ? tool : "Permission requested",
    summary: summary ? summary.replace(/\s+/g," ").slice(0,160) : typeof tool === "object" && tool !== null && !Array.isArray(tool) && typeof tool.kind === "string" ? tool.kind.replaceAll("_", " ") : "Review this action before allowing it.",
  };
}

export function toolChildId(body: unknown): string | undefined {
  if (!body || typeof body !== "object" || !("output" in body || "text" in body)) return;
  try {
    const raw = "output" in body ? body.output : "text" in body ? body.text : undefined;
    const content = Array.isArray(raw) ? raw[0]?.content?.text : raw;
    const output = typeof content === "string" ? JSON.parse(content) : content;
    if (output && typeof output.id === "string") return output.id;
  } catch { /* Non-JSON tool output has no child link. */ }
}

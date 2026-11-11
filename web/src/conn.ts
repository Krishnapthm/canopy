import { batch, createEffect } from "solid-js";
import { reconcile } from "solid-js/store";
import { state, set, ingest, items } from "./store";
import { PROTOCOL_VERSION, type Op, type ServerMsg, type Event, type Node, type Item } from "./protocol.gen";
let socket: WebSocket;
let next = 0;
let frame = 0;
let queued: Event[] = [];
const pending = new Map<number, { resolve: (v: any) => void; reject: (e: Error) => void }>();
const token = location.hash.slice(1) || sessionStorage.getItem("canopy-token") || "";
if (token) sessionStorage.setItem("canopy-token", token);
history.replaceState(null, "", location.pathname + location.search);
function call(op: Op): Promise<any> {
  if (socket?.readyState !== WebSocket.OPEN) return Promise.reject(new Error("Disconnected"));
  return new Promise((resolve, reject) => {
    const id = ++next;
    pending.set(id, {resolve, reject});
    socket.send(JSON.stringify({id, ...op}));
  });
}
export async function attempt(op: Op) {
  try { return await call(op); } catch (e) { set("error", String(e)); }
}
export async function subscribe(session: string) {
  const result = await attempt({op:"sub_session", session, after_seq:null, tail:300});
  if (result && state.selected === session) items(session, result.items as Item[], true);
}
export function connect() {
  socket = new WebSocket(`ws://${location.host}/ws`);
  socket.onopen = async () => {
    try {
      // Token belongs to the transport hello, not the shared Unix protocol.
      const request = {op:"hello" as const, version:PROTOCOL_VERSION, token};
      const hello = await call(request);
      set("harnesses", hello.harnesses);
      const tree = await call({op:"sub_tree"});
      set("nodes", reconcile(Object.fromEntries((tree.nodes as Node[]).map(n => [n.id,n]))));
      set("connected", true);
    } catch(e) { set("error", String(e)); }
  };
  socket.onmessage = message => {
    const value: ServerMsg = JSON.parse(message.data);
    if (value.t === "batch") {
      queued.push(...value.events);
      if (!frame) frame = requestAnimationFrame(() => {
        frame = 0;
        const events = queued; queued = [];
        batch(() => events.forEach(ingest));
      });
    } else {
      const request = pending.get(value.id);
      pending.delete(value.id);
      if (value.t === "err") request?.reject(new Error(value.error));
      else request?.resolve(value.result);
    }
  };
  socket.onclose = () => {
    set("connected", false);
    for (const request of pending.values()) request.reject(new Error("Disconnected"));
    pending.clear(); queued = [];
    setTimeout(connect, 1000);
  };
}
export function subscriptions() {
  let previous = "";
  createEffect(() => {
    const selected = state.selected;
    if (!state.connected) { previous = ""; return; }
    if (previous) void attempt({op:"unsub_session", session:previous});
    previous = selected;
    if (selected && !document.hidden) void subscribe(selected);
  });
  document.addEventListener("visibilitychange", () => {
    if (!state.selected || !state.connected) return;
    if (document.hidden) void attempt({op:"unsub_session", session:state.selected});
    else void subscribe(state.selected);
  });
}

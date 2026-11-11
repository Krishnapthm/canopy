import { state, set, outline } from "./store";
import { refresh, attempt } from "./conn";
export function permission(decision: "allow" | "deny" | "allow_always") {
  const item = [...(state.sessions[state.selected]?.items ?? [])].reverse().find(i => i.kind === "permission" && i.state === "streaming");
  if (item) void attempt({op:"permission", session:state.selected, request:item.id, decision});
}
export const commands = [
  {id:"refresh", title:"Refresh", key:"R", run:() => void refresh()},
  {id:"down", title:"Next agent", key:"j", run:() => move(1)},
  {id:"up", title:"Previous agent", key:"k", run:() => move(-1)},
  {id:"collapse", title:"Collapse", key:"h", run:() => set("collapsed",state.selected,true)},
  {id:"expand", title:"Expand", key:"l", run:() => set("collapsed",state.selected,false)},
  {id:"open", title:"Open session", key:"Enter", run:() => set("view","tree")},
  {id:"new", title:"New agent", key:"c", run:() => {set("child",false);set("newAgent",true);}},
  {id:"child", title:"Spawn child", key:"s", run:() => {if(state.outsideSelected) return;set("child",true);set("newAgent",true);}},
  {id:"compose", title:"Write message", key:"i", run:() => !state.outsideSelected && document.querySelector<HTMLTextAreaElement>("#composer")?.focus()},
  {id:"back", title:"Back", key:"Escape", run:() => {set("palette",false);set("newAgent",false);set("view","tree");}},
  {id:"resume", title:"Resume interrupted agent", key:"r", run:() => {if (!state.outsideSelected) void attempt({op:"resume",session:state.selected});}},
  {id:"interrupt", title:"Interrupt agent", key:"x", run:() => {if (!state.outsideSelected) void attempt({op:"interrupt",session:state.selected});}},
  {id:"allow", title:"Allow permission", key:"a", run:() => permission("allow")},
  {id:"deny", title:"Deny permission", key:"d", run:() => permission("deny")},
  {id:"filter", title:"Filter agents", key:"/", run:() => document.querySelector<HTMLInputElement>("#filter")?.focus()},
  {id:"palette", title:"Command palette", key:":", run:() => set("palette",true)},
  {id:"tree", title:"Tree view", key:"g t", run:() => set("view","tree")},
  {id:"canvas", title:"Canvas view", key:"g c", run:() => set("view","canvas")},
  {id:"plugins", title:"Plugins and skills", key:"g p", run:() => set("view","plugins")},
  {id:"help", title:"Keyboard help", key:"?", run:() => set("palette",true)},
  {id:"stop", title:"Stop agent and children", key:"", run:() => {if (!state.outsideSelected) void attempt({op:"stop",session:state.selected});}},
];
function move(delta: number) {
  const rows = outline(); const i = rows.findIndex(r => r.node.id === state.selected);
  if (rows.length) set("selected",rows[Math.max(0,Math.min(rows.length-1,i+delta))].node.id);
}
export function keys(e: KeyboardEvent) {
  if (e.key === "Escape") { commands.find(c => c.id === "back")!.run(); return; }
  if ((e.metaKey || e.ctrlKey) && e.key === "k") { e.preventDefault();set("palette",true);return; }
  if ((e.target as HTMLElement).closest("input,textarea,select,[contenteditable]")) return;
  if (e.key === "g") { prefix = true; return; }
  const key = prefix ? "g "+e.key : e.key; prefix = false;
  // Commands without a key (key "") are palette-only; an empty e.key must never run Stop.
  const command = key ? commands.find(c => c.key === key) : undefined;
  if (command) { e.preventDefault(); command.run(); }
}
let prefix = false;

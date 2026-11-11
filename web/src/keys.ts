import { state, set, outline } from "./store";
import { attempt } from "./conn";
export function permission(decision: "allow" | "deny") {
  const item = [...(state.sessions[state.selected]?.items ?? [])].reverse().find(i => i.kind === "permission" && i.state === "streaming");
  if (item) void attempt({op:"permission", session:state.selected, request:item.id, decision});
}
export const commands = [
  {id:"down", title:"Next agent", key:"j", run:() => move(1)},
  {id:"up", title:"Previous agent", key:"k", run:() => move(-1)},
  {id:"collapse", title:"Collapse", key:"h", run:() => set("collapsed",state.selected,true)},
  {id:"expand", title:"Expand", key:"l", run:() => set("collapsed",state.selected,false)},
  {id:"open", title:"Open session", key:"Enter", run:() => set("view","tree")},
  {id:"new", title:"New agent", key:"c", run:() => {set("child",false);set("newAgent",true);}},
  {id:"child", title:"Spawn child", key:"s", run:() => {set("child",true);set("newAgent",true);}},
  {id:"compose", title:"Write message", key:"i", run:() => document.querySelector<HTMLTextAreaElement>("#composer")?.focus()},
  {id:"back", title:"Back", key:"Escape", run:() => {set("palette",false);set("newAgent",false);set("view","tree");}},
  {id:"interrupt", title:"Interrupt agent", key:"x", run:() => void attempt({op:"interrupt",session:state.selected})},
  {id:"allow", title:"Allow permission", key:"a", run:() => permission("allow")},
  {id:"deny", title:"Deny permission", key:"d", run:() => permission("deny")},
  {id:"filter", title:"Filter agents", key:"/", run:() => document.querySelector<HTMLInputElement>("#filter")?.focus()},
  {id:"palette", title:"Command palette", key:":", run:() => set("palette",true)},
  {id:"tree", title:"Tree view", key:"g t", run:() => set("view","tree")},
  {id:"graph", title:"Graph view", key:"g g", run:() => set("view","graph")},
  {id:"plugins", title:"Plugins and skills", key:"g p", run:() => set("view","plugins")},
  {id:"help", title:"Keyboard help", key:"?", run:() => set("palette",true)},
  {id:"stop", title:"Stop agent and children", key:"", run:() => void attempt({op:"stop",session:state.selected})},
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
  const command = commands.find(c => c.key === key);
  if (command) { e.preventDefault(); command.run(); }
}
let prefix = false;

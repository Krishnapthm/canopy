import { useEffect, useState, lazy, Suspense } from "react";
import { useStore, set, batch } from "./store";
import { connect, subscriptions, attempt, refresh } from "./conn";
import { commands, keys } from "./keys";
import { Earth, Plus, Terminal, Network, ListTree } from "lucide-react";
import { SidebarProvider, Sidebar, SidebarHeader, SidebarContent, SidebarInset, SidebarTrigger, SidebarGroup } from "@/components/ui/sidebar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Empty, EmptyHeader, EmptyTitle, EmptyDescription, EmptyContent } from "@/components/ui/empty";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Kbd } from "@/components/ui/kbd";
import { Skeleton } from "@/components/ui/skeleton";
const Canvas = lazy(() => import("./views/Canvas"));
import Outside, { OutsideDetail } from "./views/Outside";
import Tree from "./views/Tree";
import Session from "./views/Session";
const Extensions = lazy(() => import("./views/Extensions"));
const Palette = lazy(() => import("./views/Palette"));
const NewAgent = lazy(() => import("./views/NewAgent"));
import { harnessIcon } from "./icons";
import type { HarnessDetection } from "./protocol.gen";
function restore() {
  const params = new URLSearchParams(location.search);
  batch(()=>{set("selected",params.get("session") ?? "");set("view",["tree","canvas","plugins"].includes(params.get("view") ?? "") ? params.get("view")! : "tree");});
}
restore();
subscriptions();
connect();
document.addEventListener("keydown", keys);
let previous = location.search;
window.addEventListener("popstate",()=>{previous=location.search;restore();});
export default function Workspace() {
  const state = useStore();
  // Discovery is scoped to every project on the canvas and to the selected cwd elsewhere.
  const canvas = state.view === "canvas";
  useEffect(()=>{void refresh();},[canvas]);
  const outside = state.outside.find(row=>row.vendor_id===state.outsideSelected);
  const [detected, detect] = useState<HarnessDetection[]>([]);
  useEffect(()=>{
    if (state.connected) void attempt({op:"detect"}).then(result=>{if(result) detect(result.harnesses);});
  },[state.connected]);
  useEffect(()=>{
    const params = new URLSearchParams();
    if (state.selected) params.set("session",state.selected);
    if (state.view !== "tree") params.set("view",state.view);
    const search = params.size ? "?"+params.toString() : "";
    if (search !== previous) {history.pushState(null,"",location.pathname+search);previous=search;}
  },[state.selected,state.view]);
  return <SidebarProvider><Sidebar><SidebarHeader><h1 className="flex items-center gap-2"><Earth/>Canopy</h1><Button onClick={()=>commands.find(c=>c.id==="new")!.run()}><Plus data-icon="inline-start"/>New agent</Button><Input id="filter" aria-label="Filter agents" placeholder="Filter agents /" value={state.filter} onChange={e=>set("filter",e.target.value)}/></SidebarHeader><SidebarContent><SidebarGroup><Tree/></SidebarGroup><Outside/></SidebarContent></Sidebar>
    <SidebarInset className="h-svh min-w-0"><header className="flex flex-wrap items-center gap-3 p-3"><SidebarTrigger/><Earth/>Local workspace <output>{state.connected?"Connected":"Reconnecting…"}</output><Button variant="outline" onClick={()=>set("palette",true)}><Terminal data-icon="inline-start"/>Commands<Kbd>⌘K</Kbd></Button><Tabs value={state.view} onValueChange={value=>set("view",String(value))}><TabsList><TabsTrigger value="tree"><ListTree/>Outline</TabsTrigger><TabsTrigger value="canvas"><Network/>Canvas</TabsTrigger></TabsList></Tabs></header>
    <main className="flex min-h-0 flex-1 flex-col"><Suspense fallback={<Skeleton className="m-4 h-24"/>}>
    {state.view==="plugins" ? <Extensions key={state.nodes[state.selected]?.cwd ?? "global"}/> :
    state.view==="canvas" && (Object.keys(state.nodes).length > 0 || state.outside.length > 0) ? <Canvas/> :
    outside ? <OutsideDetail row={outside}/> :
    !Object.keys(state.nodes).length ? <Empty><EmptyHeader><EmptyTitle>No agents yet</EmptyTitle><EmptyDescription>Start an agent to follow its work here.</EmptyDescription></EmptyHeader><EmptyContent>{detected.map(h=>{const Harness = harnessIcon(h.binary); return <p className="flex items-center gap-2" key={h.name}><Harness className="size-4" aria-hidden="true"/>{h.name}: {h.version}{h.logged_in === false && " - logged out - "+h.login_command}</p>;})}{state.connected && !detected.length && <p>No coding agent CLI found on your PATH.</p>}<Button onClick={()=>commands.find(c=>c.id==="new")!.run()}><Plus data-icon="inline-start"/>New agent</Button></EmptyContent></Empty> :
    <Session/>}
    </Suspense></main></SidebarInset>{/* Overlays load on demand; only they wait, never the whole app. */}<Suspense fallback={null}>{state.palette && <Palette/>}{state.newAgent && <NewAgent/>}</Suspense></SidebarProvider>;
}

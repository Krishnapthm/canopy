import { useEffect, useState, lazy, Suspense } from "react";
import { useStore, set, batch } from "./store";
import { connect, subscriptions, attempt } from "./conn";
import { commands, keys } from "./keys";
import { Earth, Plus, Terminal, Network, ListTree } from "lucide-react";
import { SidebarProvider, Sidebar, SidebarHeader, SidebarContent, SidebarInset, SidebarTrigger, SidebarGroup } from "@/components/ui/sidebar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Empty, EmptyHeader, EmptyTitle, EmptyDescription, EmptyContent } from "@/components/ui/empty";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Kbd } from "@/components/ui/kbd";
const GraphSession = lazy(() => import("./views/GraphSession"));
import Tree from "./views/Tree";
import Session from "./views/Session";
const Extensions = lazy(() => import("./views/Extensions"));
const Palette = lazy(() => import("./views/Palette"));
const NewAgent = lazy(() => import("./views/NewAgent"));
import type { HarnessDetection } from "./protocol.gen";
function restore() {
  const params = new URLSearchParams(location.search);
  batch(()=>{set("selected",params.get("session") ?? "");set("view",["tree","graph","plugins"].includes(params.get("view") ?? "") ? params.get("view")! : "tree");});
}
restore();
subscriptions();
connect();
document.addEventListener("keydown", keys);
let previous = location.search;
window.addEventListener("popstate",()=>{previous=location.search;restore();});
export default function Workspace() {
  const state = useStore();
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
  return <Suspense fallback={<p>Loading…</p>}><SidebarProvider><Sidebar><SidebarHeader><h1 className="flex items-center gap-2"><Earth/>Canopy</h1><Button onClick={()=>commands.find(c=>c.id==="new")!.run()}><Plus data-icon="inline-start"/>New agent</Button><Input id="filter" aria-label="Filter agents" placeholder="Filter agents /" value={state.filter} onChange={e=>set("filter",e.target.value)}/></SidebarHeader><SidebarContent><SidebarGroup><Tree/></SidebarGroup></SidebarContent></Sidebar>
    <SidebarInset className="h-svh min-w-0"><header className="flex flex-wrap items-center gap-3 p-3"><SidebarTrigger/><Earth/>Local workspace <output>{state.connected?"Connected":"Reconnecting…"}</output><Button variant="outline" onClick={()=>set("palette",true)}><Terminal data-icon="inline-start"/>Commands<Kbd>⌘K</Kbd></Button><Tabs value={state.view} onValueChange={value=>set("view",String(value))}><TabsList><TabsTrigger value="tree"><ListTree/>Outline</TabsTrigger><TabsTrigger value="graph"><Network/>Graph</TabsTrigger></TabsList></Tabs></header>
    <main className="flex min-h-0 flex-1 flex-col">
    {state.view==="plugins" ? <Extensions key={state.nodes[state.selected]?.cwd ?? "global"}/> :
    !Object.keys(state.nodes).length ? <Empty><EmptyHeader><EmptyTitle>No agents yet</EmptyTitle><EmptyDescription>Start an agent to follow its work here.</EmptyDescription></EmptyHeader><EmptyContent>{detected.map(h=><p key={h.name}>{h.name}: {h.version ?? "not installed"}{h.logged_in === false && " - logged out"}{(!h.version || h.logged_in === false) && " - "+h.login_command}</p>)}<Button onClick={()=>commands.find(c=>c.id==="new")!.run()}><Plus data-icon="inline-start"/>New agent</Button></EmptyContent></Empty> :
    state.view==="graph" ? <GraphSession/> : <Session/>}
    </main></SidebarInset>{state.palette && <Palette/>}{state.newAgent && <NewAgent/>}</SidebarProvider></Suspense>;
}

import { useEffect, useRef, useState, type PointerEvent } from "react";
import { useStore, set, batch, frameOf } from "../store";
import { Folder } from "lucide-react";
import { Card, CardHeader, CardContent } from "@/components/ui/card";
import { Sheet, SheetContent, SheetTitle } from "@/components/ui/sheet";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import Tree from "./Tree";
import Session from "./Session";
import { OutsideRows, OutsideDetail } from "./Outside";

type Point = {x: number; y: number};
// shortcut: layout is per browser; move it into the daemon store when clients must share it.
function savedFrames(): Record<string, Point> {
  try {
    const value: unknown = JSON.parse(localStorage.getItem("canopy.frames") ?? "{}");
    if (!value || typeof value !== "object" || Array.isArray(value)) return {};
    return Object.fromEntries(Object.entries(value).filter(([,p]) => p && typeof p === "object" && Number.isFinite(p.x) && Number.isFinite(p.y)));
  } catch { return {}; }
}
export default function Canvas() {
  const state = useStore();
  const [positions, place] = useState(savedFrames);
  const [camera, move] = useState({x: 24, y: 24, z: 1});
  const container = useRef<HTMLDivElement>(null);
  const drag = useRef<{id: number; project?: string; start?: Point; x: number; y: number} | null>(null);
  const world = useRef<HTMLDivElement>(null);
  const projects = [...new Set([...Object.values(state.nodes).map(frameOf), ...state.outside.map(r=>r.project)])].sort();
  const placed = projects.filter(p=>positions[p]), unplaced = projects.filter(p=>!positions[p]);
  // Unplaced frames flow in a column right of every placed frame (w-80 plus gap-8), so none can overlap.
  const flowLeft = placed.length ? Math.max(...placed.map(p=>positions[p].x)) + 352 : 0;
  const outside = state.outside.find(r=>r.vendor_id === state.outsideSelected);
  function zoom(factor: number, x: number, y: number) {
    move(c=>{
      const z = Math.max(.25, Math.min(2, c.z * factor));
      return {x: x-(x-c.x)*z/c.z, y: y-(y-c.y)*z/c.z, z};
    });
  }
  // The first drag pins every frame where it is, so the rest never reflow under the moving one.
  function grab(e: PointerEvent, project: string) {
    if (e.button !== 0) return;
    e.stopPropagation();
    container.current?.focus(); container.current?.setPointerCapture(e.pointerId);
    const origin = world.current!.getBoundingClientRect(), frozen: Record<string, Point> = {};
    for (const el of world.current!.querySelectorAll<HTMLElement>("[data-frame]")) {
      const r = el.getBoundingClientRect();
      frozen[el.dataset.frame!] = {x:(r.left-origin.left)/camera.z, y:(r.top-origin.top)/camera.z};
    }
    place(old=>({...frozen, ...old}));
    drag.current={id:e.pointerId,project,start:frozen[project],x:e.clientX,y:e.clientY};
  }
  function frame(project: string, at?: {left: number; top: number}) {
    return <Card data-frame={project} key={project} className={at ? "absolute w-80" : "w-80"} style={at}>
      <CardHeader className="cursor-grab touch-none" onPointerDown={e=>grab(e,project)}><Tooltip><TooltipTrigger render={<span className="flex min-w-0 items-center gap-2"/>}><Folder className="size-4 shrink-0"/><span className="truncate">{project.split("/").filter(Boolean).at(-1) || project}</span></TooltipTrigger><TooltipContent>{project}</TooltipContent></Tooltip></CardHeader>
      <CardContent className="flex flex-col gap-4">
        {Object.values(state.nodes).some(n=>frameOf(n)===project) && <Tree project={project}/>}
        {state.outside.some(r=>r.project===project) && <section><h2 className="px-2 text-xs font-medium text-muted-foreground">Running here</h2><OutsideRows project={project}/></section>}
      </CardContent>
    </Card>;
  }
  useEffect(()=>{
    const element = container.current;
    if (!element) return;
    function wheel(e: WheelEvent) {
      e.preventDefault();
      const bounds = element!.getBoundingClientRect();
      if (e.ctrlKey || e.metaKey) zoom(Math.exp(-Math.max(-10,Math.min(10,e.deltaY))*.01), e.clientX-bounds.left, e.clientY-bounds.top);
      else move(c=>({...c, x:c.x-e.deltaX, y:c.y-e.deltaY}));
    }
    element.addEventListener("wheel",wheel,{passive:false});
    return ()=>element.removeEventListener("wheel",wheel);
  },[]);
  return <>
    <div ref={container} className="relative min-h-0 flex-1 select-none overflow-clip touch-none outline-none focus-visible:ring-[3px] focus-visible:ring-inset focus-visible:ring-ring/50" tabIndex={0} aria-label="Project canvas"
      onKeyDown={e=>{
        if (e.target !== e.currentTarget) return;
        const delta: Record<string, Point> = {ArrowLeft:{x:40,y:0},ArrowRight:{x:-40,y:0},ArrowUp:{x:0,y:40},ArrowDown:{x:0,y:-40}};
        if (delta[e.key]) move(c=>({...c,x:c.x+delta[e.key].x,y:c.y+delta[e.key].y}));
        else if (e.key==="+" || e.key==="=" || e.key==="-") zoom(e.key==="-" ? 1/1.2 : 1.2,e.currentTarget.clientWidth/2,e.currentTarget.clientHeight/2);
        else if (e.key==="0") move({x:24,y:24,z:1});
        else return;
        e.preventDefault(); e.stopPropagation();
      }}
      onPointerDown={e=>{
        if (e.button !== 0 || (e.target as HTMLElement).closest("[data-frame]")) return;
        e.currentTarget.focus(); e.currentTarget.setPointerCapture(e.pointerId);
        drag.current={id:e.pointerId,x:e.clientX,y:e.clientY};
      }}
      onPointerMove={e=>{
        const d = drag.current;
        if (!d || d.id !== e.pointerId) return;
        const dx=e.clientX-d.x, dy=e.clientY-d.y;
        d.x=e.clientX; d.y=e.clientY;
        if (d.project !== undefined) {
          const project=d.project, start=d.start!, z=camera.z;
          place(old=>{const p=old[project] ?? start; return {...old,[project]:{x:p.x+dx/z,y:p.y+dy/z}};});
        } else move(c=>({...c,x:c.x+dx,y:c.y+dy}));
      }}
      onLostPointerCapture={()=>{
        if (drag.current?.project !== undefined) {
          place(latest=>{try { localStorage.setItem("canopy.frames",JSON.stringify(latest)); } catch { /* Layout can remain in memory. */ } return latest;});
        }
        drag.current=null;
      }}
      onPointerUp={e=>{if(e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);}}
      onPointerCancel={()=>{drag.current=null;}}>
      <div ref={world} className="relative" style={{transform:`translate(${camera.x}px,${camera.y}px) scale(${camera.z})`,transformOrigin:"0 0"}}>
        {placed.map(project=>frame(project,{left:positions[project].x,top:positions[project].y}))}
        <div className="absolute top-0 flex w-[1024px] flex-wrap items-start gap-8" style={{left:flowLeft}}>{unplaced.map(project=>frame(project))}</div>
      </div>
    </div>
    <Sheet open={Boolean(state.selected || outside)} onOpenChange={open=>{if(!open) batch(()=>{set("selected","");set("outsideSelected","");});}}>
      <SheetContent side="right" className="data-[side=right]:sm:max-w-xl">{/* Session and OutsideDetail carry their own visible title. */}<SheetTitle className="sr-only">{outside ? "Running here" : "Agent session"}</SheetTitle><div className="min-h-0 flex-1">{outside ? <OutsideDetail row={outside}/> : <Session/>}</div></SheetContent>
    </Sheet>
  </>;
}

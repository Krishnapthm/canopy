import { useStore, set, batch } from "../store";
import { refresh } from "../conn";
import { harnessIcon, statusIcons } from "../icons";
import { Empty, EmptyHeader, EmptyTitle, EmptyDescription } from "@/components/ui/empty";
import { RefreshCw } from "lucide-react";
import { SidebarGroup, SidebarGroupLabel, SidebarGroupAction, SidebarMenu, SidebarMenuItem, SidebarMenuButton, SidebarMenuSkeleton } from "@/components/ui/sidebar";
import type { OutsideSession } from "../protocol.gen";
// The vendor's status word, as Canopy's own words and icons.
export const outsideStatus = (row: OutsideSession) =>
  row.status === "blocked" ? {label:"Waiting for you", Icon:statusIcons.awaiting_permission}
  : row.status === "busy" ? {label:"Busy", Icon:statusIcons.running} : {label:"Idle", Icon:statusIcons.idle};
/** Read-only list of live sessions started outside Canopy (ADR 0026). */
export default function Outside() {
  const state = useStore();
  return <SidebarGroup><SidebarGroupLabel>Running here</SidebarGroupLabel>
    <SidebarGroupAction title="Refresh" aria-label="Refresh sessions running here" disabled={!state.connected || state.discovering} onClick={()=>void refresh()}><RefreshCw/></SidebarGroupAction>
    <OutsideRows/>
  </SidebarGroup>;
}

export function OutsideRows({project}: {project?: string}) {
 const state = useStore();
 return <SidebarMenu>{state.discovering ? <SidebarMenuItem><SidebarMenuSkeleton showIcon/></SidebarMenuItem> : state.outside.filter(row=>project === undefined || row.project === project).map(row=>{
      const Harness = harnessIcon(row.harness), {label, Icon} = outsideStatus(row);
      return <SidebarMenuItem key={row.vendor_id}><SidebarMenuButton className="h-auto" isActive={state.outsideSelected===row.vendor_id} onClick={()=>batch(()=>{set("selected","");set("outsideSelected",row.vendor_id);})}><Harness className="size-4" aria-hidden="true"/><div className="flex min-w-0 flex-col"><span className="truncate font-medium">{row.title || row.vendor_id.slice(-8)}</span><span className="flex items-center gap-1 text-xs text-muted-foreground"><Icon className="size-3"/>{label} · {row.vendor_id.slice(-8)}</span></div></SidebarMenuButton></SidebarMenuItem>;
    })}</SidebarMenu>;
}
export function OutsideDetail({row}: {row: OutsideSession}) {
 return <Empty><EmptyHeader><EmptyTitle>{row.title || row.vendor_id.slice(-8)}</EmptyTitle><EmptyDescription>{outsideStatus(row).label}, in {row.cwd}. Canopy shows it read-only. {row.kind === "background" ? <>Continue it with <code>claude attach {row.vendor_id.slice(0, 8)}</code>.</> : "Continue it in its terminal."}</EmptyDescription></EmptyHeader></Empty>;
}

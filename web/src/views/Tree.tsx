import { useStore, set, outline } from "../store";
import { statusIcons, harnessIcon } from "../icons";
import { SidebarMenu, SidebarMenuItem, SidebarMenuButton } from "@/components/ui/sidebar";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { Badge } from "@/components/ui/badge";
import { Folder } from "lucide-react";
export default function Tree() {
  const state = useStore();
  return <SidebarMenu>{outline().map(({node, depth}) => {
    const Status = statusIcons[node.status.state], Harness = harnessIcon(node.harness);
    return <SidebarMenuItem key={node.id}>
      <SidebarMenuButton className="h-auto" style={{paddingInlineStart: 8 + depth * 16}} isActive={state.selected === node.id} onClick={() => set("selected", node.id)}>
        <Status aria-label={node.status.state}/>
        <div className="flex min-w-0 flex-col gap-1">
          <strong className="truncate">{node.title || node.id.slice(-8)}</strong>
          <span className="flex items-center gap-2"><Harness className="size-4" aria-hidden="true"/>{node.harness} <Badge variant="outline">{node.id.slice(-8)}</Badge></span>
          <span>{node.subtree.tokens.toLocaleString()} tokens · +{node.changes.added} / -{node.changes.removed}</span>
          {node.usage.reported_cost_usd !== null && <span>${node.usage.reported_cost_usd}</span>}
          {node.cwd !== state.nodes[node.parent ?? ""]?.cwd && <Tooltip><TooltipTrigger render={<span className="flex items-center gap-2 truncate"/>}><Folder/>{node.cwd.split("/").filter(Boolean).slice(-2).join("/")}</TooltipTrigger><TooltipContent>{node.cwd}</TooltipContent></Tooltip>}
          <span className="truncate">{node.activity}</span>
        </div>
      </SidebarMenuButton>
    </SidebarMenuItem>;
  })}</SidebarMenu>;
}

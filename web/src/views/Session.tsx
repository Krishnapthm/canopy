import { useEffect, useState } from "react";
import { toolChildId, set, useStore, state as current, items, text, permissionSummary } from "../store";
import { attempt, subscribe } from "../conn";
import Markdown from "../md";
import Composer from "./Composer";
import { permission } from "../keys";
import { statusIcons, harnessIcon } from "../icons";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertTitle, AlertDescription } from "@/components/ui/alert";
import { Empty, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "@/components/ui/collapsible";
import { MessageScrollerProvider, MessageScroller, MessageScrollerViewport, MessageScrollerContent, MessageScrollerItem } from "@/components/ui/message-scroller";
import { Message, MessageContent } from "@/components/ui/message";
import { Bubble, BubbleContent } from "@/components/ui/bubble";
import { Marker } from "@/components/ui/marker";
import { Check, X, ShieldCheck, History, RefreshCw, Wrench, CircleAlert, ChevronDown } from "lucide-react";
export default function Session() {
  const state = useStore(), node = state.nodes[state.selected];
  const transcript = state.sessions[state.selected]?.items ?? [];
  const pending = transcript.filter(i=>i.kind==="permission" && i.state==="streaming").at(-1);
  const first = transcript[0]?.seq;
  const [hasEarlier, earlier] = useState(false);
  useEffect(() => {
    let active = true;
    earlier(false);
    if (state.connected && first && first > 1) {
      void attempt({op:"history",session:state.selected,before_seq:first,limit:1}).then(result => {
        if (active && result) earlier(result.items.length > 0);
      });
    }
    return () => { active = false; };
  }, [state.selected, state.connected, first]);
  if (!node) return <Empty className="h-full"><EmptyHeader><EmptyTitle>Select an agent</EmptyTitle></EmptyHeader></Empty>;
  const Status = statusIcons[node.status.state], Harness = harnessIcon(node.harness);
  return <section className="flex h-full min-h-0 flex-col" aria-label="Session">
    <header className="flex flex-wrap items-center gap-3 p-4"><h1>{node.title || node.id.slice(-8)}</h1><Badge variant="outline"><Status/>{node.status.state.replaceAll("_"," ")}</Badge><Badge variant="secondary"><Harness className="size-4" aria-hidden="true"/>{node.harness}</Badge>
      <Collapsible><CollapsibleTrigger render={<Button variant="ghost"/>}>Session details<ChevronDown data-icon="inline-end"/></CollapsibleTrigger><CollapsibleContent>{node.cwd} · {node.model || "Default model"} · {node.policy} · {node.origin}{node.changes.files.map(f=><p key={f}>{f}</p>)}</CollapsibleContent></Collapsible>
    </header>
    {/* The reason shows once: here only when the transcript does not already hold it. */}
    {node.error && !transcript.some(i=>i.kind==="error" && text(i)===node.error) && <Alert variant="destructive" className="mx-4 w-auto"><CircleAlert/><AlertDescription>{node.error}</AlertDescription></Alert>}
    {node.status.state==="awaiting_permission" && <Alert className="mx-4 w-auto"><ShieldCheck/><AlertTitle>{pending ? permissionSummary(pending).title : "Permission requested"}</AlertTitle><AlertDescription>{pending && <>{permissionSummary(pending).summary}<Collapsible><CollapsibleTrigger render={<Button variant="ghost"/>}>Full input<ChevronDown data-icon="inline-end"/></CollapsibleTrigger><CollapsibleContent><pre className="overflow-auto whitespace-pre-wrap">{JSON.stringify(pending.body,null,2)}</pre></CollapsibleContent></Collapsible></>}<div className="flex gap-2"><Button onClick={()=>permission("allow")}><Check data-icon="inline-start"/>Allow</Button><Button variant="outline" onClick={()=>permission("deny")}><X data-icon="inline-start"/>Deny</Button><Button variant="outline" onClick={()=>permission("allow_always")}><ShieldCheck data-icon="inline-start"/>Always allow</Button></div></AlertDescription></Alert>}
    <MessageScrollerProvider key={state.selected} autoScroll><MessageScroller className="h-auto flex-1"><MessageScrollerViewport><MessageScrollerContent className="p-4">
      {hasEarlier && <Button variant="outline" onClick={async()=>{
        const session = state.selected;
        const result=await attempt({op:"history",session,before_seq:transcript[0].seq,limit:300});
        if(result && current.selected === session) items(session,result.items,true);
      }}><History data-icon="inline-start"/>Earlier messages</Button>}
      <Button variant="ghost" onClick={()=>void subscribe(state.selected)}><RefreshCw data-icon="inline-start"/>Latest messages</Button>
      {transcript.map(item=><MessageScrollerItem key={item.id} className="block py-2" messageId={item.id} scrollAnchor={item.kind==="user_message"}>
        {item.kind==="error" ? <Alert variant="destructive"><CircleAlert/><AlertDescription>{text(item)}</AlertDescription></Alert> : item.kind==="notice" ? <Marker>{text(item)}</Marker> :
          item.kind==="permission" ? <Collapsible><CollapsibleTrigger render={<Button variant="ghost"/>}><ShieldCheck data-icon="inline-start"/>{permissionSummary(item).title}<ChevronDown data-icon="inline-end"/></CollapsibleTrigger><p>{permissionSummary(item).summary}</p><CollapsibleContent><pre className="overflow-auto whitespace-pre-wrap">{JSON.stringify(item.body,null,2)}</pre></CollapsibleContent></Collapsible> :
          (item.kind==="tool_call" || (item.state!=="streaming" && toolChildId(item.body))) ? <Collapsible><CollapsibleTrigger render={<Button variant="ghost" className="max-w-full"/>}><Wrench data-icon="inline-start"/><span className="truncate">{(typeof item.body === "object" && item.body !== null && !Array.isArray(item.body) ? String(item.body.title ?? "Tool") + (item.state === "streaming" ? " (running)" : " (" + item.state + ")") : text(item).slice(0,120))}</span><ChevronDown data-icon="inline-end"/></CollapsibleTrigger>{toolChildId(item.body) && <Button variant="link" onClick={()=>set("selected",toolChildId(item.body)!)}><RefreshCw data-icon="inline-start"/>Child {toolChildId(item.body)!.slice(-8)}</Button>}<CollapsibleContent><pre className="overflow-auto whitespace-pre-wrap">{JSON.stringify(item.body,null,2)}</pre></CollapsibleContent></Collapsible> :
          <Message align={item.kind==="user_message"?"end":"start"}><MessageContent><Bubble variant={item.kind==="user_message"?"secondary":"ghost"}><BubbleContent>{item.state==="streaming" ? <div className="whitespace-pre-wrap">{text(item)}</div> : <Markdown text={text(item)}/>}</BubbleContent></Bubble></MessageContent></Message>}
      </MessageScrollerItem>)}
    </MessageScrollerContent></MessageScrollerViewport></MessageScroller></MessageScrollerProvider>
    <Composer/>
  </section>;
}

import { useEffect, useState } from "react";
import { useStore, set } from "../store";
import { attempt } from "../conn";
import type { HarnessDetection, Policy } from "../protocol.gen";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { FieldGroup, Field, FieldLabel } from "@/components/ui/field";
import { Select, SelectTrigger, SelectValue, SelectContent, SelectGroup, SelectItem } from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { harnessIcon } from "../icons";
import { Plus } from "lucide-react";
export default function NewAgent() {
  const state = useStore();
  const [detected, detect] = useState<HarnessDetection[]>([]);
  const [harness, choose] = useState(state.harnesses[0] ?? "");
  const [policy, permissions] = useState<Policy>("ask");
  const [busy, pending] = useState(false);
  useEffect(()=>{void attempt({op:"detect"}).then(result=>{if(result) detect(result.harnesses);});},[]);
  const options = [...new Set([...state.harnesses, ...detected.map(h=>h.name)])];
  const unavailable = (name: string) => !state.harnesses.includes(name) || detected.some(h=>h.name === name && h.logged_in === false);
  return <Dialog open onOpenChange={open=>set("newAgent",open)}><DialogContent><DialogHeader><DialogTitle>{state.child?"Spawn child":"New agent"}</DialogTitle><DialogDescription>Launch an installed harness in a working directory.</DialogDescription></DialogHeader>
    <form onSubmit={async e=>{
      e.preventDefault(); if (busy) return; const data=new FormData(e.currentTarget); pending(true);
      const result=await attempt({op:"spawn",harness,prompt:String(data.get("prompt")),cwd:String(data.get("cwd")),model:String(data.get("model"))||null,policy,parent:state.child?state.selected||null:null,title:null});
      pending(false); if(result){set("selected",result.session);set("newAgent",false);}
    }}><FieldGroup>
      <Field><FieldLabel>Harness</FieldLabel><Select value={harness} onValueChange={value=>choose(value ?? "")}><SelectTrigger aria-label="Harness"><SelectValue/></SelectTrigger><SelectContent><SelectGroup>{options.map(h=>{const info = detected.find(row=>row.name===h), Harness = harnessIcon(info?.binary ?? h); return <SelectItem key={h} value={h} disabled={unavailable(h)}><Harness className="size-4" aria-hidden="true"/>{h}{info && (!info.version || info.logged_in === false) ? " - "+(!info.version ? "not installed" : "logged out")+" - "+info.login_command : ""}</SelectItem>;})}</SelectGroup></SelectContent></Select>
      {detected.map(h=><p key={h.name}>{h.name}: {!h.version ? "not installed" : h.logged_in === false ? "logged out" : h.version}{(!h.version || h.logged_in === false) && ` - ${h.login_command}`}</p>)}</Field>
      <Field><FieldLabel htmlFor="cwd">Working directory</FieldLabel><Input id="cwd" name="cwd" required placeholder="/path/to/project" defaultValue={state.nodes[state.selected]?.cwd ?? ""}/></Field>
      <Field><FieldLabel htmlFor="model">Model</FieldLabel><Input id="model" name="model" placeholder="Harness default"/></Field>
      <Field><FieldLabel>Permissions</FieldLabel><Select value={policy} onValueChange={value=>permissions(value as Policy)}><SelectTrigger aria-label="Permissions"><SelectValue/></SelectTrigger><SelectContent><SelectGroup>{(["ask","auto_edit","yolo"] as const).map(p=><SelectItem key={p} value={p}>{p.replaceAll("_"," ")}</SelectItem>)}</SelectGroup></SelectContent></Select></Field>
      <Field><FieldLabel htmlFor="prompt">Task</FieldLabel><Textarea id="prompt" name="prompt" required autoFocus/></Field>
      <Button disabled={!state.connected || !harness || unavailable(harness) || busy}><Plus data-icon="inline-start"/>Start agent</Button>
    </FieldGroup></form>
  </DialogContent></Dialog>;
}

import { useEffect, useState } from "react";
import { useStore, set } from "../store";
import { attempt, call } from "../conn";
import type { HarnessDetection, Policy } from "../protocol.gen";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { FieldGroup, Field, FieldLabel, FieldError } from "@/components/ui/field";
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
  const [error, fail] = useState("");
  const [busy, pending] = useState(false);
  useEffect(()=>{void attempt({op:"detect"}).then(result=>{if(result) detect(result.harnesses);});},[]);
  // The daemon lists only installed harnesses; detection rows match by binary and add labels and login state.
  const options = state.harnesses;
  const unavailable = (name: string) => !state.harnesses.includes(name) || detected.some(h=>h.binary === name && h.logged_in === false);
  const info = detected.find(h=>h.binary === harness);
  const reason = !state.connected ? "Not connected" : !options.length ? "" : !harness ? "Pick a harness" : unavailable(harness) ? "Logged out - "+(info?.login_command ?? "log in with its CLI") : busy ? "Starting agent" : "";
  return <Dialog open onOpenChange={open=>set("newAgent",open)}><DialogContent><DialogHeader><DialogTitle>{state.child?"Spawn child":"New agent"}</DialogTitle><DialogDescription>Launch an installed harness in a working directory.</DialogDescription></DialogHeader>
    <form onSubmit={async e=>{
      e.preventDefault(); if (busy) return; const data=new FormData(e.currentTarget); pending(true);
      fail("");
      try { const result=await call({op:"spawn",harness,prompt:String(data.get("prompt")),cwd:String(data.get("cwd")),model:String(data.get("model"))||null,policy,parent:state.child?state.selected||null:null,title:null});
      if(result){set("selected",result.session);set("newAgent",false);}
      } catch(error) { fail(error instanceof Error ? error.message : String(error)); } finally { pending(false); }
    }}><FieldGroup>
      <Field><FieldLabel>Harness</FieldLabel><Select items={Object.fromEntries(options.map(h=>[h, detected.find(row=>row.binary===h)?.name ?? h]))} value={harness} onValueChange={value=>choose(value ?? "")}><SelectTrigger aria-label="Harness"><SelectValue/></SelectTrigger><SelectContent><SelectGroup>{options.map(h=>{const info = detected.find(row=>row.binary===h), Harness = harnessIcon(h); return <SelectItem key={h} value={h} disabled={unavailable(h)}><Harness className="size-4" aria-hidden="true"/>{info?.name ?? h}{info?.logged_in === false ? " - logged out - "+info.login_command : ""}</SelectItem>;})}</SelectGroup></SelectContent></Select>
      {!options.length && <p>No coding agent CLI found on your PATH.</p>}</Field>
      <Field><FieldLabel htmlFor="cwd">Working directory</FieldLabel><Input id="cwd" name="cwd" required placeholder="/path/to/project" defaultValue={state.nodes[state.selected]?.cwd ?? ""}/></Field>
      <Field><FieldLabel htmlFor="model">Model</FieldLabel><Input id="model" name="model" placeholder="Harness default"/></Field>
      <Field><FieldLabel>Permissions</FieldLabel><Select value={policy} onValueChange={value=>permissions(value as Policy)}><SelectTrigger aria-label="Permissions"><SelectValue/></SelectTrigger><SelectContent><SelectGroup>{(["ask","auto_edit","yolo"] as const).map(p=><SelectItem key={p} value={p}>{p.replaceAll("_"," ")}</SelectItem>)}</SelectGroup></SelectContent></Select></Field>
      <Field><FieldLabel htmlFor="prompt">Task</FieldLabel><Textarea id="prompt" name="prompt" required autoFocus/></Field>
      <Button type="submit" disabled={!state.connected || !harness || unavailable(harness) || busy}><Plus data-icon="inline-start"/>Start agent</Button>
      {reason && <p>{reason}</p>}{error && <FieldError>{error}</FieldError>}
    </FieldGroup></form>
  </DialogContent></Dialog>;
}

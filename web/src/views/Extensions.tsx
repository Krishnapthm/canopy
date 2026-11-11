import { useEffect, useState } from "react";
import type { Extension } from "../protocol.gen";
import { useStore } from "../store";
import { errorToast } from "../toast";
import { attempt } from "../conn";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Field, FieldGroup, FieldLabel, FieldDescription } from "@/components/ui/field";
import { Empty, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Earth, Folder, Download, RefreshCw } from "lucide-react";
export default function Extensions() {
  const state = useStore();
  const cwd = state.nodes[state.selected]?.cwd ?? null;
  const [rows, setRows] = useState<Extension[]>([]);
  const [busy, setBusy] = useState(false);
  async function refresh() {
    setRows([]);
    const result = await attempt({op:"ext_list", cwd});
    if (result) { setRows(result.items); result.errors.forEach((error: string) => errorToast(error)); }
  }
  useEffect(() => {
    let active = true;
    setRows([]); setBusy(true);
    void attempt({op:"ext_list", cwd}).then(result => {
      if (active && result) { setRows(result.items); result.errors.forEach((error: string) => errorToast(error)); }
    }).finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, [cwd, state.connected]);
  async function change(row: Extension, adopt = false) {
    setBusy(true);
    try { await attempt({op:"ext_set", harness:row.harness, kind:adopt ? "adopt_skill" : row.kind, ext_id:row.id, scope:row.scope, enabled:adopt || !row.enabled}); }
    finally {
      await refresh();
      setBusy(false);
    }
  }
  return <section className="flex min-h-0 flex-col gap-4 overflow-auto p-4" aria-label="Plugins and skills">
    <h1>Plugins and skills</h1>
    <p>Running agents are not affected. Applies to new sessions.</p>
    <Button variant="outline" disabled={busy || !state.connected} onClick={() => { setBusy(true); void refresh().finally(()=>setBusy(false)); }}><RefreshCw data-icon="inline-start"/>Refresh</Button>
    {busy && ["Claude", "Codex"].map(harness => <div key={harness} role="status">{harness}<Skeleton className="h-8 w-full"/></div>)}
    {([['plugin','Plugins'],['skill','Skills'],['library_skill','Library skills']] as const).map(([kind,title]) => {
      const groups = new Map<string, Extension[]>();
      for (const row of rows.filter(r=>r.kind===kind)) {
        const key = JSON.stringify([row.name,row.scope]);
        groups.set(key,[...(groups.get(key) ?? []),row]);
      }
      return <FieldGroup key={kind}><h2>{title}</h2>{[...groups].map(([key, entries]) => <FieldGroup key={key}>
        <h3>{entries[0].name} <Badge variant="outline">{entries[0].scope.scope === "global" ? <><Earth/>Global</> : <><Folder/>{entries[0].scope.cwd}</>}</Badge></h3>
        {entries.map(row=><Field key={row.harness+row.id} orientation="horizontal">
          <FieldLabel>{row.harness}</FieldLabel><Switch aria-label={`${row.name} for ${row.harness}`} checked={row.enabled} disabled={busy || !state.connected || !row.writable || !!row.command} onCheckedChange={()=>void change(row)}/>
          {kind==="skill" && !row.command && <Button variant="outline" disabled={busy || !state.connected} onClick={()=>void change(row,true)}><Download data-icon="inline-start"/>Adopt {row.name} from {row.harness}</Button>}
          {row.command && <FieldDescription>Marketplace command (run manually): <code className="whitespace-pre-wrap break-all">{row.command}</code></FieldDescription>}
        </Field>)}
      </FieldGroup>)}</FieldGroup>;
    })}
    {!busy && !rows.length && <Empty><EmptyHeader><EmptyTitle>No plugins or skills found</EmptyTitle></EmptyHeader></Empty>}
  </section>;
}

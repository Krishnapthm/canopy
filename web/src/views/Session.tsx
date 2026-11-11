import { For, Show } from "solid-js";
import { state, items, text } from "../store";
import { attempt, subscribe } from "../conn";
import Markdown from "../md";
import Composer from "./Composer";
import { permission } from "../keys";
import { Icon } from "../icons";
export default function Session() {
  const node = () => state.nodes[state.selected];
  const transcript = () => state.sessions[state.selected]?.items ?? [];
  return <section class="session" aria-label="Session"><Show when={node()} fallback={<div class="empty"><Icon name="tree"/><h2>Your agents, together.</h2><p>Start an agent to follow its work here.</p></div>}>{n => <>
    <header><h2>{n().title || n().id.slice(-8)}</h2><p>{n().harness} · {n().cwd} · {n().status.state}</p><details><summary>Session details</summary><p>{n().model || "Default model"} · {n().policy} · {n().origin}</p><For each={n().changes.files}>{f=><div>{f}</div>}</For></details></header>
    <Show when={n().status.state==="awaiting_permission"}><aside role="alert">Permission requested <button onClick={()=>permission("allow")}><Icon name="status"/>Allow</button><button onClick={()=>permission("deny")}><Icon name="stop"/>Deny</button></aside></Show>
    <div class="transcript"><button onClick={async()=>{
      const result=await attempt({op:"history",session:state.selected,before_seq:transcript()[0]?.seq ?? Number.MAX_SAFE_INTEGER,limit:300});
      if(result) items(state.selected,result.items,true);
    }}><Icon/>Earlier messages</button><button onClick={()=>void subscribe(state.selected)}><Icon/>Latest messages</button>
    <For each={transcript()}>{item=><article><small><Icon/>{item.kind.replaceAll("_"," ")}</small><Show when={item.state!=="streaming"} fallback={<div class="stream">{text(item)}</div>}><Markdown text={text(item)}/></Show></article>}</For></div>
    <Composer/>
  </>}</Show></section>;
}

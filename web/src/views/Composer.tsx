import { createSignal } from "solid-js";
import { state, set } from "../store";
import { attempt } from "../conn";
import { Icon } from "../icons";
export default function Composer() {
  const [value, write] = createSignal("");
  return <form onSubmit={async e => {
    e.preventDefault(); if (!value().trim()) return;
    const result = await attempt({op:"send",session:state.selected,text:value(),interrupt:false});
    if(result) {set("delivery",result.delivery);write("");}
  }}>
    <textarea id="composer" aria-label="Message agent" placeholder="Send a follow-up…" value={value()} onInput={e=>write(e.currentTarget.value)} disabled={!state.connected || state.nodes[state.selected]?.origin==="native"}/>
    <button disabled={!state.selected || state.nodes[state.selected]?.origin==="native"}><Icon/>Send</button><output>{state.delivery}</output>
  </form>;
}

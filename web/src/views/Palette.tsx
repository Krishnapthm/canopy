import { For, createSignal } from "solid-js";
import { commands } from "../keys";
import { set } from "../store";
import { Icon } from "../icons";
export default function Palette() {
  const [query,search]=createSignal("");
  return <div class="overlay"><section role="dialog" aria-modal="true" aria-label="Commands"><input autofocus aria-label="Find command" placeholder="Find a command…" onInput={e=>search(e.currentTarget.value)}/><For each={commands.filter(c=>c.title.toLowerCase().includes(query().toLowerCase()))}>{c=><button onClick={()=>{set("palette",false);c.run();}}><Icon/>{c.title}<kbd>{c.key}</kbd></button>}</For><button onClick={()=>set("palette",false)}>Close</button></section></div>;
}

import { For } from "solid-js";
import { state, set, outline } from "../store";
import { Icon } from "../icons";
export default function Tree() {
  return <nav aria-label="Agents"><For each={outline()}>{({node,depth}) =>
    <button class="agent" classList={{selected:state.selected===node.id}} style={{"padding-left":(12+depth*16)+"px"}} onClick={() => set("selected",node.id)}>
      <span><Icon name="status"/><strong>{node.title || node.id.slice(-8)}</strong></span>
      <span><Icon/>{node.harness} · {node.id.slice(-8)} · {node.status.state}</span>
      <small>{node.activity || node.cwd}</small>
      <small>{node.subtree.tokens.toLocaleString()} tokens · +{node.changes.added} / -{node.changes.removed}{node.usage.reported_cost_usd !== null ? " · $"+node.usage.reported_cost_usd : ""}</small>
    </button>
  }</For></nav>;
}

import { For } from "solid-js";
import { outline, set } from "../store";
export default function Graph() {
  const rows = outline;
  return <svg class="graph" role="img" aria-label="Agent tree graph" viewBox={"0 0 900 "+Math.max(200,rows().length*64)}>
    <For each={rows()}>{(row,index) => {
      const x = () => 30+row.depth*200, y = () => 32+index()*64;
      const parent = () => rows().findIndex(r => r.node.id === row.node.parent);
      return <g>
        {parent() >= 0 && <path d={`M${x()-200},${32+parent()*64}H${x()-100}V${y()}H${x()}`}/>}
        <a tabindex="0" role="button" aria-label={"Select "+row.node.title} onClick={() => set("selected",row.node.id)} onKeyDown={e => {if(e.key==="Enter") set("selected",row.node.id);}}>
          <circle class={row.node.status.state} cx={x()} cy={y()} r="6"/>
          <text x={x()+12} y={y()}>{row.node.title || row.node.id.slice(-8)}</text>
          <text class="activity" x={x()+12} y={y()+18}>{row.node.activity.slice(0,26)}</text>
        </a>
      </g>;
    }}</For>
  </svg>;
}

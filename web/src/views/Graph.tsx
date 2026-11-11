import { outline, set, useStore } from "../store";
import { statusIcons } from "../icons";
export default function Graph() {
  useStore();
  const rows = outline();
  return <svg className="size-full min-h-64" role="img" aria-label="Agent tree graph" viewBox={`0 0 ${Math.max(900, ...rows.map(r=>r.depth*240+300))} ${Math.max(200,rows.length*64)}`}>
    {rows.map((row,index) => {
      const x = 30+row.depth*240, y = 32+index*64;
      const parent = rows.findIndex(r => r.node.id === row.node.parent);
      const Status = statusIcons[row.node.status.state];
      return <g key={row.node.id}>
        {parent >= 0 && <path fill="none" stroke="currentColor" d={`M${x-240},${32+parent*64}H${x-120}V${y}H${x}`}/>}
        <g tabIndex={0} role="button" aria-label={"Select "+row.node.title} onClick={() => set("selected",row.node.id)} onKeyDown={e => {if(e.key==="Enter" || e.key===" ") {e.preventDefault();set("selected",row.node.id);}}}>
          <Status x={x-8} y={y-12} width={20} height={20}/>
          <text fill="currentColor" fontSize={16} x={x+20} y={y}>{row.node.title || row.node.id.slice(-8)}</text>
          <text fill="currentColor" fontSize={16} x={x+20} y={y+20}>{row.node.activity.slice(0,26)}</text>
        </g>
      </g>;
    })}
  </svg>;
}

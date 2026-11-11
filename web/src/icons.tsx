export function Sprite() {
  return <svg aria-hidden="true" style={{display:"none"}}><defs>
    <symbol id="tree" viewBox="0 0 16 16"><path d="M3 1v11h10M3 5h10M7 5v7M10 3l3 2-3 2M10 10l3 2-3 2"/></symbol>
    <symbol id="graph" viewBox="0 0 16 16"><path d="M4 8h7M4 8l7-5M4 8l7 5"/><circle cx="3" cy="8" r="2"/><circle cx="13" cy="3" r="2"/><circle cx="13" cy="13" r="2"/></symbol>
    <symbol id="globe" viewBox="0 0 16 16"><circle cx="8" cy="8" r="7"/><path d="m3 3 4 1 1 3-3 1-1 3-2-3m9-5-2 4 3 2-1 4-3-2-1-4"/></symbol>
    <symbol id="plus" viewBox="0 0 16 16"><path d="M8 2v12M2 8h12"/></symbol>
    <symbol id="status" viewBox="0 0 16 16"><circle cx="8" cy="8" r="5"/><path d="m5 8 2 2 4-4"/></symbol>
    <symbol id="command" viewBox="0 0 16 16"><path d="m5 3 5 5-5 5M11 13h3"/></symbol>
    <symbol id="stop" viewBox="0 0 16 16"><path d="M3 3h10v10H3z"/></symbol>
  </defs></svg>;
}
export function Icon(props: {name?: string}) {
  return <svg class="icon" aria-hidden="true"><use href={"#"+(props.name ?? "command")}/></svg>;
}

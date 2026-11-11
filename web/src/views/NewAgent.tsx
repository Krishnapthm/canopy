import { For } from "solid-js";
import { state, set } from "../store";
import { attempt } from "../conn";
import { Icon } from "../icons";
import type { Policy } from "../protocol.gen";
export default function NewAgent() {
  return <div class="overlay"><form role="dialog" aria-modal="true" aria-label="New agent" onSubmit={async e=>{
    e.preventDefault();const data=new FormData(e.currentTarget);
    const result=await attempt({op:"spawn",harness:String(data.get("harness")),prompt:String(data.get("prompt")),cwd:String(data.get("cwd")),model:String(data.get("model"))||null,policy:data.get("policy") as Policy,parent:state.child?state.selected||null:null,title:null});
    if(result){set("selected",result.session);set("newAgent",false);}
  }}><h2>{state.child?"Spawn child":"New agent"}</h2>
    <label>Harness<select name="harness"><For each={state.harnesses}>{h=><option>{h}</option>}</For></select></label>
    <label>Working directory<input name="cwd" required placeholder="/path/to/project" value={state.nodes[state.selected]?.cwd ?? ""}/></label>
    <label>Model<input name="model" placeholder="Harness default"/></label>
    <label>Permissions<select name="policy"><option value="ask">Ask</option><option value="auto_edit">Auto edit</option><option value="yolo">Allow everything</option></select></label>
    <label>Task<textarea name="prompt" required autofocus/></label>
    <button disabled={!state.connected}><Icon name="plus"/>Start agent</button><button type="button" onClick={()=>set("newAgent",false)}>Cancel</button>
  </form></div>;
}

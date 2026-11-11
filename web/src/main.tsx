import { render } from "solid-js/web";
import { Show, onMount, createEffect, batch } from "solid-js";
import { state, set } from "./store";
import { connect, subscriptions } from "./conn";
import { commands, keys } from "./keys";
import { Sprite, Icon } from "./icons";
import Tree from "./views/Tree";
import Graph from "./views/Graph";
import Session from "./views/Session";
import Palette from "./views/Palette";
import NewAgent from "./views/NewAgent";
import "./styles.css";
function App() {
  const restore = () => {
    const params = new URLSearchParams(location.search);
    batch(() => {
      set("selected", params.get("session") ?? "");
      set("view", ["tree","graph","plugins"].includes(params.get("view") ?? "") ? params.get("view")! : "tree");
    });
  };
  restore();
  let previous = location.search;
  createEffect(() => {
    const params = new URLSearchParams();
    if (state.selected) params.set("session",state.selected);
    if (state.view !== "tree") params.set("view",state.view);
    const search = params.size ? "?"+params.toString() : "";
    if (search !== previous) {
      history.pushState(null,"",location.pathname+search);
      previous = search;
    }
  });
  window.addEventListener("popstate",()=>{previous=location.search;restore();});
  subscriptions();
  onMount(()=>{connect();document.addEventListener("keydown",keys);});
  return <><Sprite/><header class="top"><h1><Icon name="tree"/>Canopy</h1><span><Icon name="globe"/>Local workspace</span><output>{state.connected?"Connected":"Reconnecting…"}</output><button onClick={()=>set("palette",true)}><Icon/>Commands <kbd>⌘K</kbd></button></header>
    <Show when={state.error}><aside role="alert">{state.error}<button onClick={()=>set("error","")}>Dismiss</button></aside></Show>
    <div class="workspace"><aside class="sidebar"><div class="tools"><button onClick={()=>commands.find(c=>c.id==="new")!.run()}><Icon name="plus"/>New agent</button><button onClick={()=>set("view",state.view==="graph"?"tree":"graph")}><Icon name="graph"/>View</button></div><input id="filter" aria-label="Filter agents" placeholder="Filter agents /" value={state.filter} onInput={e=>set("filter",e.currentTarget.value)}/><Tree/></aside>
    <main><Show when={state.view!=="plugins"} fallback={<section class="empty"><Icon name="globe"/><h2>Plugins and skills</h2><p>Plugin and skill controls arrive in M5.5.</p></section>}><Show when={state.view==="graph"}><Graph/></Show><Session/></Show></main></div>
    <Show when={state.palette}><Palette/></Show><Show when={state.newAgent}><NewAgent/></Show>
  </>;
}
render(()=> <App/>,document.getElementById("app")!);

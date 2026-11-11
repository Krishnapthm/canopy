import assert from "node:assert/strict";
import {state,items,ingest,outline,set,listen,batch,permissionSummary,toolChildId,newlyFailed} from "./src/store.ts";
const item = {id:"one",session:"session",seq:1,body:{text:"a"},state:"streaming",kind:"agent_message"};
items("session",[item]);
ingest({t:"delta",session:"session",item:"one",text:"b"});
assert.equal(state.sessions.session.items[0].body.text,"ab");
ingest({t:"item",item:{...item,body:{text:"complete"},state:"complete"}});
assert.equal(state.sessions.session.items.length,1);
assert.equal(state.sessions.session.items[0].body.text,"complete");
items("session",Array.from({length:350},(_,i)=>({...item,id:String(i),seq:i})),true);
assert.equal(state.sessions.session.items.length,300);
assert.equal(state.sessions.session.items[0].seq,50);
items("session",[item],true);
assert.equal(state.sessions.session.items.length,1);
set("nodes",{root:{id:"root",parent:null,created:0,title:"Root"},child:{id:"child",parent:"root",created:1,title:"Child"}});
assert.deepEqual(outline().map(r=>r.depth),[0,1]);
set("collapsed","root",true);
assert.equal(outline().length,1);
console.log("Web store: streaming replacement, bounded snapshots, outline collapse passed");

const before = state;
let notifications = 0;
const unlisten = listen(() => notifications++);
batch(() => { set("filter", "root"); set("filter", ""); });
assert.equal(notifications, 1);
assert.notEqual(state, before);
assert.equal(before.filter, "");
unlisten();
set("filter", "test");
assert.equal(notifications, 1);
console.log("Web store: immutable snapshots, batched notifications and unsubscribe passed");

// Exercise the transport snapshot path, including a selected idle session on reconnect.
const callbacks = {};
globalThis.location = {hash:"",pathname:"/",search:"",host:"localhost"};
globalThis.sessionStorage = {getItem:()=>null};
globalThis.history = {replaceState:()=>{}};
// sonner injects its stylesheet on import.
globalThis.document = {hidden:false,addEventListener:(name,fn)=>callbacks[name]=fn,head:{appendChild(){}},createElement:()=>({appendChild(){}}),createTextNode:()=>({})};
let ws;
const discoveries = [];
const outside = {harness:"claude",vendor_id:"outside12345678",cwd:"/tmp/project",title:"Outside fixture",kind:"interactive",started:0,status:"busy",pid:123};
globalThis.WebSocket = class {
  static OPEN = 1;
  readyState = 1;
  constructor() { ws = this; }
  send(raw) {
    const request = JSON.parse(raw);
    if (request.op === "discover") discoveries.push(request.cwd);
    const result = request.op === "hello" ? {harnesses:["fake"]} : request.op === "sub_tree" ? {nodes:[]} : request.op === "discover" ? {sessions:[outside],errors:[]} : {items:[
      {...item,id:"user",kind:"user_message",body:{text:"plan"}},
      {...item,id:"agent",kind:"agent_message",body:{text:"Planning a **small** change"}},
      {...item,id:"tool",kind:"tool_call",body:{title:"spawn_agent",input:{}}},
    ]};
    queueMicrotask(()=>this.onmessage({data:JSON.stringify({t:"res",id:request.id,result})}));
  }
};
const { registerHooks } = await import("node:module");
registerHooks({ resolve(specifier, context, nextResolve) {
  if (context.parentURL?.includes("/web/src/") && specifier.startsWith("./") && !specifier.endsWith(".ts")) specifier += ".ts";
  return nextResolve(specifier, context);
} });
const {connect,subscriptions,refresh} = await import("./src/conn.ts");
set("selected","idle-root");
subscriptions(); connect(); await ws.onopen();
await new Promise(resolve=>setTimeout(resolve,0));
assert.deepEqual(state.sessions["idle-root"].items.map(i=>i.kind),["user_message","agent_message","tool_call"]);
assert.deepEqual(state.outside,[outside]);
assert.equal(state.discovering,false);
assert.deepEqual(discoveries,[null]);
set("nodes",{"idle-root":{cwd:"/tmp/project"}});
await refresh();
assert.deepEqual(discoveries,[null,"/tmp/project"]);
console.log("Web transport: idle root subscription snapshot passed");

assert.deepEqual(permissionSummary({...item,body:{title:"Write config.toml",tool:{kind:"execute",rawInput:{secret:"hidden"}}}}),{title:"Write config.toml",summary:"secret: hidden"});
assert.deepEqual(permissionSummary({...item,body:{title:"Bash",input:{command:"cargo test\n--offline"}}}),{title:"Bash",summary:"command: cargo test --offline"});

// Exercise the actual React token renderer, including nested inline list tokens.
const {readFile, writeFile, unlink} = await import('node:fs/promises');
const {transformWithOxc} = await import('vite');
const {marked} = await import('marked');
const {createElement} = await import('react');
const {renderToStaticMarkup} = await import('react-dom/server');
const rendererPath = new URL('./.markdown-check.mjs', import.meta.url);
try {
  const source = await readFile(new URL('./src/md.tsx', import.meta.url), 'utf8');
  await writeFile(rendererPath, (await transformWithOxc(source + '\nexport {Tokens};', 'md.tsx', {jsx:{runtime:'automatic'}})).code);
  const {Tokens} = await import(rendererPath.href);
  const html = renderToStaticMarkup(createElement(Tokens,{tokens:marked.lexer('A **small** change\n\n- **one**\n- `two`\n\n<script>alert(1)</script>')}));
  assert.ok(html.includes('<strong>small</strong>'));
  assert.ok(html.includes('<li><strong>one</strong></li>'));
  assert.ok(html.includes('<code>two</code>'));
  assert.ok(!html.includes('<script>'));
} finally { await unlink(rendererPath).catch(()=>{}); }
console.log('Markdown: completed paragraphs, nested lists, code and escaped HTML passed');

assert.equal(toolChildId({output:'{"id":"child"}'}),'child');
assert.equal(toolChildId({output:[{content:{text:'{"id":"acp-child"}'}}]}),'acp-child');
assert.equal(toolChildId({text:'{"id":"demo-child"}'}),'demo-child');
assert.equal(toolChildId({output:'normal output'}),undefined);

const failedNode = {id:"agent",status:{state:"failed"}};
set("nodes",{});
assert.equal(newlyFailed(failedNode),true,"an agent that fails before its first event still counts");
set("nodes",{agent:failedNode});
assert.equal(newlyFailed(failedNode),false,"history loaded on connect is silent");
set("nodes",{agent:{...failedNode,status:{state:"running"}}});
assert.equal(newlyFailed(failedNode),true);
ingest({t:"node",node:failedNode});
assert.equal(newlyFailed(failedNode),false,"a repeated failed event is silent");
console.log("Web store: failure transitions passed");

set("outsideSelected","outside12345678");
set("selected","owned");
assert.equal(state.outsideSelected,"");

import { createResource, For, Show, type JSX } from "solid-js";
import type { Token } from "marked";
function safe(url: string) {
  try { const u = new URL(url, location.href); return ["http:","https:","mailto:"].includes(u.protocol) ? u.href : undefined; }
  catch { return undefined; }
}
function Tokens(props: { tokens: Token[] }): JSX.Element {
  return <For each={props.tokens}>{token => {
    const t = token as Token & {tokens?: Token[]; text?: string; href?: string; items?: {tokens: Token[]}[]};
    const children = () => t.tokens ? <Tokens tokens={t.tokens}/> : t.text;
    switch(t.type) {
      case "paragraph": return <p>{children()}</p>;
      case "heading": return <h3>{children()}</h3>;
      case "strong": return <strong>{children()}</strong>;
      case "em": return <em>{children()}</em>;
      case "codespan": return <code>{t.text}</code>;
      case "code": return <pre><code>{t.text}</code></pre>;
      case "blockquote": return <blockquote>{children()}</blockquote>;
      case "list": return <ul><For each={t.items}>{i => <li><Tokens tokens={i.tokens}/></li>}</For></ul>;
      case "link": return <a href={safe(t.href ?? "")} target="_blank" rel="noreferrer">{children()}</a>;
      case "br": return <br/>;
      case "space": return null;
      default: return <span>{t.text ?? t.raw}</span>;
    }
  }}</For>;
}
export default function Markdown(props: {text: string}) {
  const [tokens] = createResource(() => props.text, async text => (await import("marked")).marked.lexer(text));
  return <Show when={tokens()} fallback={<span>{props.text}</span>}>{t => <Tokens tokens={t()}/>}</Show>;
}

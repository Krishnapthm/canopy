import { useEffect, useState, Fragment, type ReactNode } from "react";
import type { Token } from "marked";
function safe(url: string) {
  try { const u = new URL(url, location.href); return ["http:","https:","mailto:"].includes(u.protocol) ? u.href : undefined; }
  catch { return undefined; }
}
function Tokens(props: { tokens: Token[] }): ReactNode {
  return props.tokens.map(token => {
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
      case "list": return <ul>{t.items?.map((i, index) => <li key={index}><Tokens tokens={i.tokens}/></li>)}</ul>;
      case "link": return <a href={safe(t.href ?? "")} target="_blank" rel="noreferrer">{children()}</a>;
      case "br": return <br/>;
      case "space": return null;
      default: return <span>{t.text ?? t.raw}</span>;
    }
  }).map((child, index) => <Fragment key={index}>{child}</Fragment>);
}
export default function Markdown(props: {text: string}) {
  const [tokens, setTokens] = useState<Token[]>();
  useEffect(() => {
    let active = true;
    setTokens(undefined);
    void import("marked").then(({marked}) => { if (active) setTokens(marked.lexer(props.text)); });
    return () => { active = false; };
  }, [props.text]);
  return tokens ? <Tokens tokens={tokens}/> : <span>{props.text}</span>;
}

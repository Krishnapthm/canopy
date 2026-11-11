import { useState } from "react";
import { useStore, set } from "../store";
import { attempt } from "../conn";
import { Send } from "lucide-react";
import { InputGroup, InputGroupTextarea, InputGroupAddon, InputGroupButton } from "@/components/ui/input-group";
export default function Composer() {
  const state = useStore();
  const [value, write] = useState("");
  const [sending, pending] = useState(false);
  const disabled = !state.connected || state.nodes[state.selected]?.origin === "native" || sending;
  return <form className="p-4" onSubmit={async e => {
    e.preventDefault(); if (!value.trim() || disabled) return;
    pending(true);
    const result = await attempt({op:"send",session:state.selected,text:value,interrupt:false});
    pending(false);
    if(result) {set("delivery",result.delivery);write("");}
  }}>
    <InputGroup><InputGroupTextarea id="composer" aria-label="Message agent" placeholder="Send a follow-up…" value={value} onChange={e=>write(e.target.value)} disabled={disabled}/>
      <InputGroupAddon align="block-end"><output>{state.delivery.replaceAll("_", " ")}</output><InputGroupButton type="submit" disabled={disabled || !value.trim()}><Send data-icon="inline-start"/>Send</InputGroupButton></InputGroupAddon>
    </InputGroup>
  </form>;
}

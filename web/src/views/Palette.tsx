import { commands } from "../keys";
import { useStore, set } from "../store";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { Command, CommandInput, CommandList, CommandEmpty, CommandGroup, CommandItem } from "@/components/ui/command";
import { Kbd } from "@/components/ui/kbd";
import { Terminal } from "lucide-react";
export default function Palette() {
  const state = useStore();
  return <Dialog open={state.palette} onOpenChange={open=>set("palette",open)}><DialogContent>
    <DialogHeader><DialogTitle>Commands and keyboard help</DialogTitle><DialogDescription>Find an action or its keyboard shortcut.</DialogDescription></DialogHeader>
    <Command><CommandInput placeholder="Find command…"/><CommandList><CommandEmpty>No matching commands.</CommandEmpty><CommandGroup>
      {commands.map(c=><CommandItem key={c.id} onSelect={()=>{set("palette",false);c.run();}}><Terminal/>{c.title}<Kbd>{c.id === "palette" ? "⌘K / :" : c.key}</Kbd></CommandItem>)}
    </CommandGroup></CommandList></Command>
  </DialogContent></Dialog>;
}

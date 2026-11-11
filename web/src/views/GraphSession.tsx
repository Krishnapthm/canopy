import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "@/components/ui/resizable";
import Graph from "./Graph";
import Session from "./Session";
export default function GraphSession() {
  return <ResizablePanelGroup orientation="vertical"><ResizablePanel><Graph/></ResizablePanel><ResizableHandle/><ResizablePanel><Session/></ResizablePanel></ResizablePanelGroup>;
}

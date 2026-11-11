import { createRoot } from "react-dom/client";
import { Toaster } from "./components/ui/sonner";
import Workspace from "./Workspace";
import { ThemeProvider } from "next-themes";
import "./index.css";
createRoot(document.getElementById("app")!).render(
  <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
    <Workspace/>
    <Toaster closeButton/>
  </ThemeProvider>
);

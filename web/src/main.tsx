import { createRoot } from "react-dom/client";
import Workspace from "./Workspace";
import { ThemeProvider } from "next-themes";
import "./index.css";
createRoot(document.getElementById("app")!).render(
  <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
    <Workspace/>
  </ThemeProvider>
);

import { createRoot } from "react-dom/client";
import { lazy, Suspense } from "react";
import { ThemeProvider } from "next-themes";
import "./index.css";
const Workspace = lazy(() => import("./Workspace"));
createRoot(document.getElementById("app")!).render(
  <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
    <Suspense fallback={<p role="status">Loading Canopy…</p>}><Workspace/></Suspense>
  </ThemeProvider>
);

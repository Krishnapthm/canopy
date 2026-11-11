import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
// Explicit dev target, copied from canopy web's printed URL (without fragment).
const target = process.env.CANOPY_WEB_ORIGIN ?? "http://127.0.0.1:7331";
const proxy = {
  target, ws: true, changeOrigin: true,
  configure(server: any) {
    const origin = (request: any) => request.setHeader("Origin", target);
    server.on("proxyReqWs", origin);
  },
};
export default defineConfig({
  plugins: [solid()],
  server: { host: "127.0.0.1", proxy: { "/ws": proxy } },
  build: { target: "es2022" },
});

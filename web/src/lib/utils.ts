// Tables fitted to the classes in web/src, generated at build time by the cn/vite plugin
// (vite.config.ts). The full "cn" tables cost ~26 KB of first-load JS.
import tables from "./cn-tables"
import { createCn } from "cn/engine"

export const cn = createCn(tables)

import { Loader, Play, Coffee, ShieldQuestion, Timer, Check, CircleX, Square, Pause, Terminal, Bot, Code } from "lucide-react";
import type { Status } from "./protocol.gen";
export const statusIcons = { starting: Loader, running: Play, idle: Coffee, awaiting_permission: ShieldQuestion, rate_limited: Timer, done: Check, failed: CircleX, cancelled: Square, interrupted: Pause } satisfies Record<Status["state"], unknown>;
export const harnessIcon = (harness: string) => harness === "claude" ? Bot : harness === "codex" ? Code : Terminal;

import ClaudeCode from "@lobehub/icons/es/ClaudeCode/components/Mono.js";
import Codex from "@lobehub/icons/es/Codex/components/Mono.js";
import { Loader, Play, Coffee, ShieldQuestion, Timer, Check, CircleX, Square, Pause, Terminal } from "lucide-react";
import type { Status } from "./protocol.gen";
export const statusIcons = { starting: Loader, running: Play, idle: Coffee, awaiting_permission: ShieldQuestion, rate_limited: Timer, done: Check, failed: CircleX, cancelled: Square, interrupted: Pause } satisfies Record<Status["state"], unknown>;
// shortcut: brand icons only for harnesses Canopy runs; others (OpenCode, Copilot, Gemini) cost
// ~2.4 KB gzip of first-load budget. Add their Mono icons when Canopy can run them (ADR 0027).
export const harnessIcon = (harness: string) => {
  switch (harness) {
    case "claude": case "Claude Code": return ClaudeCode;
    case "codex": case "Codex": return Codex;
    default: return Terminal;
  }
};

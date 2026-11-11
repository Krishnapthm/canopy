# Web

- Wire types come from Rust via make gen; never edit src/protocol.gen.ts.
- State belongs in src/store.ts. React reads immutable snapshots with useSyncExternalStore.
- src/conn.ts batches event ingestion with requestAnimationFrame and store batch.
- Drop session subscriptions while hidden; fetch a fresh bounded snapshot on focus.
- Render streaming text directly. Only completed items enter the lazy Markdown lexer.
- src/md.tsx renders React elements. Never use innerHTML or HTML token injection.
- Keep transcript windows at 300 items. Earlier messages use protocol history paging.
- Compose installed shadcn components in src/components/ui; do not build replacement controls.
- Use lucide icons: Earth for global scope, Folder for project scope, distinct status shapes.
- Keep the generated neutral theme in src/index.css; no added colors or color status cues.
- Use FieldGroup/Field for forms, CommandGroup for commands, SelectGroup for options.
- Dialogs need titles; button icons use data-icon. Empty states use Empty.
- Chat uses MessageScroller, Message, Bubble and Marker. No eyebrow labels.
- Use gap, size and cn for layout; do not modify UI components except for type fixes.
- ADR 0022 approves the stack. No router, state library or additional dependencies.
- Run make web for production assets and the 120 KB initial JS gzip assertion.
- build.mjs traverses static manifest imports, excluding lazy workspace, views and Markdown.
- Build assets before cargo install or release builds. make build does both.
- For development, start a foreground daemon with a temporary CANOPY_HOME.
- Copy the printed web URL origin into CANOPY_WEB_ORIGIN, then run npm exec vite.
- Stop both temporary processes before continuing. Never use a live harness for checks.

## Browser performance check

1. Build assets and binary. Use a temporary home, never the user's home.
2. Start canopy daemon --foreground with --max-live 50 and an ACP fake-agent command.
3. Launch 50 fake agents with repeated say and sleep 10 commands for 30 seconds.
4. Open canopy web, select a streaming session and record Chrome Performance.
5. Require 60 fps, no main-thread task above 50 ms and JS heap at most 60 MB.
6. Repeat outline and graph, light and dark, narrow and wide windows.
7. Hide the tab: transcript subscriptions stop. Focus: snapshot resyncs.
8. Restart the daemon in the same temp home: reconnect and resync without reload.
9. Stop temporary processes and delete only that temporary home.

Browser checks require a visible browser. The initial gzip budget excludes lazy chunks;
it does not measure total workspace transfer, frame rate, accessibility or heap.

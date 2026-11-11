# Web

- Wire types come from Rust via make gen; never edit src/protocol.gen.ts.
- State belongs in src/store.ts. Use Solid stores and signals, no other state library.
- src/conn.ts batches event ingestion with requestAnimationFrame and Solid batch.
- Drop session subscriptions while hidden; fetch a fresh bounded snapshot on focus.
- Render streaming text directly. Only completed items enter the lazy Markdown lexer.
- src/md.tsx renders tokens as DOM nodes. Never use innerHTML or HTML token injection.
- Keep transcript windows at 300 items. Earlier messages use protocol history paging.
- Icons use src/icons.tsx, currentColor, a 16 px viewBox; global scope is the earth.
- Keep styles in src/styles.css with light and dark custom properties.
- Run make web for production assets, gzip files and the 50 KB total JS gzip assertion.
- Build assets before cargo install or release builds. make build does both.
- For development, start a foreground daemon with a temporary CANOPY_HOME.
- Copy the printed web URL origin into CANOPY_WEB_ORIGIN, then run npm exec vite.
- Stop both temporary processes before continuing. Never use a live harness for checks.

## Browser performance check

1. Run make web and build the binary. Use a temporary home, never the user's home.
2. Start canopy daemon --foreground with --max-live 50 and an ACP fake-agent command.
3. Launch 50 fake agents with a prompt repeating say and sleep 10 for 30 seconds.
   Generate the prompt with Python: print("say chunk\nsleep 10\n" * 3000).
   Use canopy run with that prompt and the configured fake harness, never a vendor.
4. Open canopy web against that temporary home; select one streaming session.
5. Record Chrome Performance for 30 seconds in both tree and graph views.
6. Require 60 fps, no main-thread task above 50 ms, and JS heap at most 60 MB.
7. Repeat in light and dark themes, and inspect narrow and wide windows.
8. Hide the tab: transcript subscriptions must stop. Focus: snapshot must resync.
9. Kill and restart the daemon in the same temporary home. The page must reconnect
   with the saved token, a fresh tree snapshot and selected transcript without reload.
10. Stop the daemon and browser/dev servers. Delete only that temporary home.

These browser checks require a visible browser and remain manual. No browser budget
result should be inferred from the gzip build assertion.

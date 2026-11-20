# Brief: write the wiki page {{PAGE}}

You write one page of Canopy's user documentation: `docs/wiki/{{PAGE}}.md`.
Nothing else. Other sessions write the other pages at the same time.

## Rules

- Work in `{{ROOT}}`. Create or rewrite only `docs/wiki/{{PAGE}}.md`. Do not edit, move or delete any other file.
- Do not commit, push, build, or start the app, a core, `scripts/dev` or any agent. Run only read-only commands:
  read files, `git show origin/main:<path>`, `grep`, `{{BIN}} --help`, `{{BIN}} commands`, `hc --help`, `hc <verb> --help`.
- Follow ponytail mode from `{{PONYTAIL}}`: the shortest page that covers everything in your brief.
- Every fact on the page must come from the source or the old doc, and you must check it in the source.
  When the old doc and the source disagree, the source wins. If you cannot verify a fact, leave it off the page.
- Do not invent buttons, keys, commands or limits. Spell UI labels exactly as the source draws them.

## Read first, in this order

1. `docs/canopy/doc-style.md`: the style. Follow every rule.
2. `docs/wiki/Glossary.md`: the only words to use for Canopy things.
3. `docs/wiki/Simulator-Devices.md`: the gold-standard page. Match its tone, length and shape.
4. `docs/canopy/doc-owners.md`: what every page owns. Do not cover a fact another page owns. Link to it.
5. Your block below, then its sources.

## Your block

{{BLOCK}}

## Done when

- `bun scripts/doc-style.ts docs/wiki/{{PAGE}}.md` prints "Style clean".
- The page fits 20 to 80 lines, and every heading in "Headings" is present, in that order.
- Every link uses `[text](Page-Name)` with a page name from `docs/wiki/_Sidebar.md`, or a full URL.
- Each image line from "Shots" is in place with its exact file name and alt text.

## Final report (your last message)

1. **Claims:** each fact on the page, with the `file:line` that proves it.
2. **Left out:** facts you could not verify, and why.
3. **Gaps:** behaviour you saw in the source that no page in the owner map owns.
Keep the report short. Do not paste the page.

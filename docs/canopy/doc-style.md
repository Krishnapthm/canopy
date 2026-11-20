# Wiki style

How the pages in `docs/wiki/` are written. These are principles, not quotas:
when one gets in the way of a reader understanding something, the reader wins.
`bun scripts/doc-style.ts` checks the few mechanical rules (filler words, em
dashes, portable Markdown). Review checks everything else.

## Who the pages are for

A developer opening Canopy for the first time. They know terminals, Git and at
least one coding agent CLI. They do not know what a space, a wire or a quote is,
or why they would want one. They read a page to get something done, and they
come back later to go deeper.

Write as an experienced developer showing a colleague around: plain, direct,
specific, a little warm. Not a spec, not a sales page.

## Principles

**Purpose before mechanics.** Open with what the thing is for and when you
would reach for it, then how to use it, then the details. "A wire introduces
two agents so they can talk to each other" comes before "drag the **+** handle".

**Introduce a term before you lean on it.** The first time a page uses a
Canopy word (space, canvas mode, wire, quote, herdr-core), say what it means in
a clause or link to the Concepts page. Use the words in the Glossary page
exactly, and only with that meaning. A pane is always a pane, never a card,
cell, tile or window.

**Show it.** A realistic example beats an abstract description. Use real
commands, real prompts and real output: an agent reviewing a diff, a dev server
on `localhost:3000`, a quote as the agent receives it. Say what the reader
should see after a step, so they know it worked.

**Explain enough to predict the app.** A sentence of "how it works" is worth
keeping when it saves a surprise ("closing the window does not stop your
agents, because herdr-core runs them"). Internals that never change what the
reader does or sees stay in the code.

**Be honest about limits, where they bite.** Put a limit next to the feature
it limits ("macOS only", "files up to 1 MiB"). Never drop a true condition to
make a sentence nicer. Describe the app as it is today: no "will", "soon" or
"for now". A missing feature is a plain statement: "Rotation is not supported."

**Match depth to the page.** A concept or guide may run long, with paragraphs
that build an idea. A reference page is a table you scan. Do not pad either.
Length follows what the reader needs, not a budget.

**Keep facts in one home, but do not make readers hop.** Each fact has an
owner page ([doc-owners.md](doc-owners.md)) that states it in full. Other pages
may restate it in a sentence when that saves a click, and link to the owner for
the rest. A restatement uses the same values, never a variation.

**Write clearly, not tersely.** Prefer active voice, present tense and "you".
Vary sentence length. One idea per paragraph. Use verbs over nominalizations
("search the PDF", not "perform a search"). Cut filler: simply, easily,
seamless, powerful, robust, leverage, utilize, "note that". No em dashes:
use a plain dash, a comma or two sentences.

## Page types

Every page is one of these. The sidebar groups them.

- **Tutorial** (Quick-Start). A guided path with one outcome. Numbered steps,
  each followed by what you should see. Explain only what the next step needs.
  End with what the reader learned and where to go next.
- **Concept** (Concepts). Builds the mental model. Prose, diagrams, examples.
- **Guide** (Pair-Agents, Point-Agents-at-Things, ...). A real goal from start
  to finish: the situation, the steps, what the agents see, variations, what
  to watch out for. Links down to feature pages for the controls it uses.
- **Feature** (Canvas, Browser-Pane, ...). One part of the app. Shape:
  1. What it is and when you use it, then a screenshot.
  2. Task sections named by what you do ("Open a page", "Pick an element").
  3. Details and limits a power user needs.
  4. Troubleshooting, as **symptom.** then cause and fix.
  5. Related pages.
- **Reference** (Keys-and-Mouse, CLI, Commands, Files-and-Folders, Labels,
  Glossary). Tables and short lists. Terse is right here. One line of context
  at the top so a reader knows they are in the right place.

Leave out a section with nothing real in it.

## Formatting

- UI labels in **bold**, spelled exactly as the app draws them: **Connect**, **Tile view**.
- Commands, keys, paths, settings and env vars in `code`: `canopy wire`, `Cmd+K`, `~/.config/canopy`.
- Keys: macOS first, then the others: `Cmd+T` (`Ctrl+Shift+T` on Linux and Windows).
- Shell examples go in `sh` fences, with a `#` comment when a line needs one.
  `bun scripts/docs.ts check` checks every `canopy` command in a page against
  the binary, so examples must be real.
- What an agent receives goes in a `text` fence, exactly as Canopy produces it.
- Warnings go before the step they apply to: `> **Warning:** Deleting a space ends its terminals.`
- Use tables for reference data with three or more rows (keys, settings, buttons).
- Diagrams are Mermaid code fences, which GitHub renders. Keep them small: a
  diagram that needs a legend is too big.

## Portable Markdown

The wiki may move to Fumadocs later, so pages stay plain:

- Line 1 is `# Title`. The next paragraph says what the page is about.
- CommonMark, GFM tables and code fences only. No raw HTML, no `[[wiki links]]`, no `> [!NOTE]` alerts.
- Internal links: `[text](Page-Name)` or `[text](Page-Name#heading)`.
  Repo files: a full `https://github.com/Krishnapthm/canopy/blob/main/...` URL.
- Images: `![What the image shows](images/<page>-<n>.png)`, always with alt text
  that describes what the image shows.
- Headings are anchors. Renaming one means updating every link to it.
- Code links to some pages by name (`grep -rn 'wiki/' src defaults`). Do not
  rename those pages.

## Before and after

From the rewrite that introduced this guide:

- Before: "A wire introduces two panes as peers. It does not carry messages or
  limit who can message whom."
  After: "A wire is an introduction. When you wire two agents, each can find
  out who the other is and how to reach it, so you can tell one 'ask the
  reviewer to check this' and it knows who you mean."
- Before: "Canopy wraps pane text in an envelope. The text cannot close the
  envelope. Canopy never runs it."
  After: "Canopy wraps what an agent reads from a pane in an envelope that marks
  it as data, so a web page that says 'ignore your instructions' cannot pass
  itself off as you."
- Before: "1. If **Teach agents Canopy** opens, press `Esc`."
  After: "Canopy offers to add a short note to your project's `AGENTS.md`.
  Accept it: that note is how your agents learn they can open panes and talk
  to each other."

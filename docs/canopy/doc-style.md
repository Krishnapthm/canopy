# Wiki style

How every page in `docs/wiki/` is written. The rules come from ASD-STE100
(Simplified Technical English), adapted for user docs. `bun scripts/doc-style.ts`
checks the mechanical rules. Review checks the rest.

## Substance first

- Every line tells the reader something they need. No intros, recaps or filler.
- A short page that covers everything beats a long one. Budget: 20 to 80 lines.
- Prefer numbered steps and bullets to paragraphs.
- Each fact lives on one page only. Other pages link to it. `docs/canopy/doc-owners.md` says which page owns what.
- Write for a person using the app. Leave out internals that do not change what they do.
  Keep a limit or a rule when it changes what they see ("files up to 512 KiB").

## Sentences (ASD-STE100)

- Instructions: imperative, one action per sentence, 20 words or fewer.
- Descriptions: 25 words or fewer per sentence.
- A paragraph has 3 sentences or fewer and one topic.
- Steps go in a numbered list, 7 steps or fewer. Split a longer task into two lists under two headings.
- Use active voice and simple present tense. Keep articles (a, the).
- Use verbs, not nouns made from verbs: "search the PDF", not "perform a search".
- Do not use phrasal verbs: start, not spin up. Configure, not set up (as a verb). Open, not bring up.
- Keep noun clusters to 3 words or fewer.
- Keep a true condition or limit ("macOS only", "may"). Never drop one to make a sentence shorter.
- No semicolons and no em dashes. Write two sentences.

## Words

- One word, one meaning. Use the terms in [Glossary](../wiki/Glossary.md) exactly.
  A pane is always a pane: never a card, cell, tile or window.
- Use the same verb for the same action on every page: click, drag, press, type, select, open, close, wire.
- Banned: simply, just, easily, seamless, powerful, robust, leverage, utilize, please, note that, currently, soon.
- Write "you" for the person. Do not write "we" or "I".
- Say what is true now. Do not write "will", "for now" or "not yet" about the app.
  Write a missing feature as a plain limit: "Rotation is not supported."

## Formatting

- UI labels in **bold**, spelled exactly as drawn: **Connect**, **Tile view**.
- Commands, keys, paths, settings and env vars in `code`: `canopy wire`, `Cmd+K`, `~/.config/canopy`.
- Keys: macOS first, then the other platforms in brackets: `Cmd+K` (`Ctrl+Shift+K` on Linux and Windows).
- Shell examples go in `sh` code fences with a short `#` comment per line when the line needs one.
- Warnings come before the step they apply to: `> **Warning:** Closing a note deletes its text.`
- Tables only for reference data with 3 or more rows (keys, settings).

## Portable Markdown (for a later Fumadocs move)

- Line 1 is `# Title`. The next paragraph is the description: one or two sentences.
- Only CommonMark, GFM tables and code fences. No raw HTML, no `[[wiki links]]`, no `> [!NOTE]` alerts.
- Internal links have one form: `[text](Page-Name)` or `[text](Page-Name#heading)`.
  Links to repo files use a full `https://github.com/Krishnapthm/canopy/blob/main/...` URL.
- Images: `![What the image shows](images/<page>-<n>.png)`, alt text always.
- Headings are anchors: do not rename one without updating every link to it.

## Page template

```markdown
# <Name>

<What it is and why you use it, in one or two sentences.>

![<alt>](images/<page>-1.png)

## Use it

1. <Step.>
2. <Step.>

<CLI example in a sh fence, when there is one.>

## How it works

<At most 2 short paragraphs. Only what helps the reader predict the app.>

## Troubleshooting

- **<What the person sees.>** <Cause.> <Fix.>

## See also

- [<Page>](<Page-Name>)
```

Leave out a section with nothing real in it. A page may use task headings
("## Open a PDF", "## Highlight text") in place of one "Use it" section.

## Before and after

- "Pane content is wrapped in an envelope, which it cannot close early, and it is never run." becomes
  "Canopy wraps pane text in an envelope. The text cannot close the envelope. Canopy never runs it."
- "You can easily spin up a new agent by clicking Agent." becomes "Click **Agent** to start an agent."
- "Perform a drag on the + handle to initiate a connection." becomes "Drag the **+** handle to another pane."

# Point Agents at Things

Half of working with an agent is explaining what you are looking at: "the function around line 40", "the blue button under the price", "the error on the second page". In Canopy you point instead. Select the thing in its pane, click the quote icon, and Canopy puts a precise reference into the agent's prompt. This guide shows how that works for each kind of pane and what the agent actually receives.

## How a quote works

Every quote follows the same three steps:

1. **Select** something in a pane: lines of text, a region of an image, a passage in a PDF, an element on a web page, or marks on a simulator screen.
2. **Click the quote icon** that appears next to the selection. The quote menu opens.
3. **Pick an agent.** Optionally type a request in **Add a message (optional)** first, such as `Why is this slow?`.

![The quote menu listing wired agents first](images/quotes-1.png)

The menu lists agents wired to the pane you quoted from first, then your other running agents, then **New agent with this**, which starts a fresh agent with the quote. Canopy pastes the quote and your message into the agent's prompt but does not press `Enter`. You can read it, add to it, and send it when you are ready.

The quote is a reference, not a copy of the whole file. File paths are written relative to the agent's folder when possible, so the agent can open the file itself and see the context.

## Lines in a file

Open a file on the canvas (**Open file** on the toolbar, or `Cmd+O` / `Ctrl+O`), or preview it in the [Files panel](Files-Panel). Select some lines and click the quote icon. The agent receives the path, the line range and the text:

````text
@src/api/login.rs:40-42
```rs
fn check_password(user: &User, given: &str) -> bool {
    user.password == given
}
```

Use a constant-time comparison here.
````

Markdown files are quoted as `>` lines instead of a code block.

**When to use it:** code review comments, "explain this", "this is where the bug is", or pointing an agent at the exact paragraph of a design doc you want it to follow.

## A region of an image

Open a screenshot or design mock as an image pane. Drag a box over the part you care about (at least 4 by 4 pixels), then click the quote icon. To quote the whole image, click **Quote image** in the helper bar, or select the pane and press `q`.

A region quote names the image, the region in source pixels, and a cropped copy Canopy saved for the agent to look at:

```text
@designs/checkout.png region 120,340 480x96 /tmp/canopy/p4-region-120-340-480x96.png
```

**When to use it:** "make the header look like this", or showing an agent the one part of a bug report screenshot that matters.

## A passage in a PDF

In a PDF pane, click **Highlight text** in the helper bar (or press `m`), drag across the text, and click the quote icon. The agent receives the file, the page and the passage:

```text
@specs/payments-api.pdf page 12
> Refunds must be issued within 30 days of the original charge.
```

Your highlights stay on the PDF pane, so you can click one later to quote it again.

**When to use it:** implementing against a spec, an RFC or a vendor's API reference.

## An element on a web page

Open your dev server in a browser pane on macOS. Click **Annotate**, hover an element and click it. A numbered pin drops on it and a comment box opens. Type something like “This card overflows on mobile” and press `Enter` to queue the comment, or `Cmd+Enter` to send it right away. Drag from text to comment on a range, or drag elsewhere to comment on an area. [Browser Pane](Browser-Pane#annotate) describes the controls.

The agent receives the screenshot with the pins on it, and a message that points at the element rather than copying it. For a React development build it also names the component and its source file, so the agent can go straight to the code:

```text
Annotated http://localhost:3000/pricing (viewport 1280x800)
Image 1 is the annotated screenshot; numbered blue dots mark the comments.

1. This card overflows on mobile
   Element: main > section:nth-of-type(2) > div.card <div class="card"> "Pro plan"
   Component: PricingCard (/work/app/src/components/PricingCard.tsx:42)
```

**When to use it:** "this button is misaligned on mobile", "this card should show the annual price", or any UI fix where finding the right component is half the work. [Browser Pane](Browser-Pane#how-canopy-finds-the-source-file) explains how Canopy finds the source file.

## A spot on the simulator

In an iOS simulator pane, click **Annotate** in the bottom bar. Click the button with the problem and save a comment like “Increase the tap target.” Click **Send**. The agent receives the screenshot and a message such as:

```text
Annotated iPhone 17 (iOS 26.1), 402x874 points
Image 1 is the annotated screenshot; numbered blue dots mark the comments.

1. Increase the tap target
   Point: x 120, y 300
```

For problems that span several controls, drag an area or draw an arrow. [Simulator Devices](Simulator-Devices#annotate) explains how to add a crop or use the live simulator while annotating.

## Terminals are different

Selecting text in a terminal pane copies it to the clipboard, like any terminal. It does not open the quote menu. To show an agent another terminal's output, either paste it in, or let the agent read the terminal itself with `hc read` (see [Wiring](Wiring#talk-to-peers)).

## A complete loop: fix a UI bug

Here is how the pieces combine when you work on a web app:

1. An agent runs your dev server and opens it next to itself: `canopy pane new browser localhost:3000 --wire`. (See [Let Agents Show You](Let-Agents-Show-You).)
2. Click **Annotate** and click the broken element. Save a comment such as `This overflows on narrow screens`.
3. Click **Send**. The wired agent receives the screenshot and the element details, including the React component and source location when available.
4. When it says it is done, it can take a screenshot of the browser pane with `canopy pane shot` to check its own work, or you click **Reload** and look.

## See also

- [Quotes](Quotes): the quote menu and every quote format in one place.
- [Images and PDFs](Images-and-PDFs), [Browser Pane](Browser-Pane), [Simulator Devices](Simulator-Devices): each pane kind in detail.
- [Concepts](Concepts#quotes-pointing-at-things): where quotes fit among Canopy's ideas.

# Quotes

A quote takes something you selected in a pane and puts a precise reference to it into an agent's prompt, so you can say "fix this" and point instead of describe. You can add a message, and Canopy never presses `Enter` for you: you see exactly what the agent will get before you send it.

This page is the reference for the quote menu and quote formats. For examples of quoting from each kind of pane, read [Point Agents at Things](Point-Agents-at-Things).

## Quote a selection

1. Select something in a pane. The quote icon appears next to the selection.
2. Click the quote icon. The quote menu opens.
3. Optionally type a request in **Add a message (optional)**.
4. Click an agent.

What you can select depends on the pane:

| Pane | How to select |
|---|---|
| File, or a preview in the Files panel | Select text. |
| Image | Drag a region, or click **Quote image** for the whole image. See [Images and PDFs](Images-and-PDFs). |
| PDF | Turn on **Highlight text**, then drag across text. See [Images and PDFs](Images-and-PDFs). |
| Browser | Click **Pick an element to quote**, then an element. See [Browser Pane](Browser-Pane#pick-an-element-to-quote). |
| Simulator | Click **Annotate**, draw, then **Send to agent**. See [Simulator Devices](Simulator-Devices). |
| Terminal | Not supported. Selecting terminal text copies it. |

## Choose the agent

![The quote menu listing wired agents first](images/quotes-1.png)

The quote menu lists agents in this order:

1. **Wired to this pane:** agents wired to the pane you quoted from. See [Wiring](Wiring).
2. Your other running agents.
3. **New agent with this:** your installed agents. Clicking one starts a new agent in the current space with the quote in its prompt.

Canopy expands the chosen agent's pane and pastes the quote. It waits up to 30 seconds for the agent to be ready to receive a paste. If the agent is not ready in time, nothing is sent.

## What the agent receives

Every quote starts with `@` and the source: a file path (relative to the agent's folder when possible), or a URL. Your message, if you typed one, comes after a blank line.

| Source | Format |
|---|---|
| File lines | `@path:start-end`, then the text in a code block. Markdown files use `>` lines instead. |
| Image region | `@path region x,y WxH crop-path`, where `crop-path` is a saved copy of the region. |
| Whole image | `@path` |
| PDF | `@path page N`, then the passage as `>` lines. |
| Browser element | `@url`, then the component and source file and lines when known, else a CSS selector, then the element's opening tag or text. |
| Simulator | `@path` of the annotated screenshot, then a line naming the device and its size. |

For example, three lines of a Rust file with a message:

````text
@src/api/login.rs:40-42
```rs
fn check_password(user: &User, given: &str) -> bool {
    user.password == given
}
```

Use a constant-time comparison here.
````

Review the prompt, then press `Enter` in the agent to send it.

## See also

- [Point Agents at Things](Point-Agents-at-Things): examples for every pane kind.
- [Concepts](Concepts#quotes-pointing-at-things): where quotes fit among Canopy's ideas.

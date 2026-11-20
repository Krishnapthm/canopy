# Quotes

A quote puts a selection into an agent's input without pressing `Enter`.
You can add a message before you choose the agent.

## Quote a selection

1. Select text in a file pane, a region in an image pane, or text with PDF highlighting enabled.
2. Click the quote icon that appears after the selection.

- Terminals: selections copy text. They do not open the quote menu. See [Terminals](Terminals).
- Files: select text in the pane or preview. See [Files Panel](Files-Panel).
- Images: quote a region or the whole image. See [Images and PDFs](Images-and-PDFs).
- PDFs: select text with **Highlight text** enabled. See [Images and PDFs](Images-and-PDFs).
- Browser: click **Pick an element to quote**, then an element. See [Browser Pane](Browser-Pane).
- Simulator: use **Annotate**, then **Send to agent**. See [Simulator Devices](Simulator-Devices).

## Who gets the quote

![The quote menu listing wired agents first](images/quotes-1.png)

The quote menu lists these choices in order:

1. Agents under **Wired to this pane**, when present. See [Wiring](Wiring).
2. Other running agents.
3. Installed agents under **New agent with this**.

Type in **Add a message (optional)** if you want to add a request.
Click an agent to paste the quote, or start one with the quote.

## What the agent receives

File quotes include the path, selected text and source line numbers when available.
Canopy adds your optional message after the quote, separated by a blank line.

Canopy pastes into the agent's input without pressing `Enter`.
Review the input before you press `Enter` to submit it.

# Images and PDFs

Image and PDF panes bring reference material onto the canvas: a screenshot from a bug report, a design mock, an API spec, a paper. Their main job is to let you point agents at a precise part of them, a region of an image or a passage of a PDF, instead of describing it.

Open either with **Open file** on the toolbar, `Cmd+O` (`Ctrl+O` on Linux and Windows), or by dragging the file onto the canvas.

On macOS, choose **Open in Preview** in the helper bar to open the original file in Apple Preview.

## Images

image panes show PNG, JPEG, GIF and WebP files. a new pane keeps the image's proportions and shrinks large images to fit the visible canvas at 100% canvas zoom. small images keep their original pixel size at 100%, with extra space when the pane reaches its minimum size. reopening an existing pane keeps its size.

### Quote part of an image

1. Drag a box over the part you want, at least 4 by 4 pixels of the source image.
2. Click the quote icon that appears.
3. Pick an agent in the [quote menu](Quotes).

Canopy saves the cropped region to a temporary file and gives the agent both the original path and the crop, so it can look at exactly what you selected.

To quote the whole image, click **Quote image** in the helper bar, or select the pane and press `q` in canvas mode.

The image helper bar also has **Connect** and **Close**.

## PDFs

![A PDF pane with highlighted text](images/images-and-pdfs-1.png)

PDF panes let you read, search, highlight and quote a document. a new pane uses the first page's proportions to fit the visible canvas at 100% canvas zoom. the PDF starts in fit-width mode, which adjusts the page when you resize the pane. reopening an existing pane keeps its size.

zooming the canvas scales the page with the pane, like a picture, so you see the same part of the PDF at any canvas zoom. when the canvas eases to a new zoom, the first page bitmap is drawn for the zoom where the canvas will come to rest.

### Find text

1. Click **Find** in the helper bar, or press `/` with the pane selected.
2. Click the PDF so it has the keyboard.
3. Type what you are looking for.

### Highlight and quote

1. Click **Highlight text** in the helper bar, or press `m`.
2. Drag across the text you want.
3. Click the quote icon to send it to an agent with the file name and page number.

Highlights stay on the pane, so you can mark up a spec as you read it and quote each passage when you need it. Click a saved highlight to bring its quote icon back. Highlights are saved in `canvas.json`, never in the PDF file itself.

### Move around

- Double-click the PDF to enter it, then scroll with the wheel or two fingers. Click empty canvas or another pane to leave it. Until you enter it, scrolling over the PDF pans the canvas.
- Click **Zoom out**, **Reset zoom**, or **Zoom in** to change the PDF zoom. Reset returns the page to fit the pane's width.
- Click **Previous page** or **Next page**, or press `[` and `]` with the pane selected.
- Click **Clear highlights**, or press `Shift+M`, to remove every highlight. The button shows only when there are some.

The PDF helper bar also has **Connect** and **Close**.

### Keys

With a PDF pane selected in canvas mode:

| Key | Action |
|---|---|
| `/` | Turn **Find** on or off. |
| `m` | Turn **Highlight text** on or off. |
| `[` / `]` | Previous / next page. |
| `Shift+M` | Clear all highlights. |

## What agents can read

An agent that reads an image pane with `canopy pane read` gets its path and pixel size, so it can open the file itself. For a PDF pane, it gets the path, the page count and the text with page numbers. See [Agent Pane Commands](Agent-Pane-Commands#read-a-pane).

## Troubleshooting

- **No quote icon appears after selecting a region.** The region is too small. Drag a box at least 4 by 4 pixels of the source image.
- **Typing does not search the PDF.** Click **Find**, then click the PDF itself before typing.
- **A toast says "The PDF is password protected".** Canopy cannot open password-protected PDFs.

## See also

- [Point Agents at Things](Point-Agents-at-Things): quotes from every pane kind, with examples.
- [Quotes](Quotes): the quote menu and formats.

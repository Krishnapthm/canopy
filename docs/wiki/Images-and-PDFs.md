# Images and PDFs

Image panes show a whole picture. PDF panes let you read, search and highlight a PDF.

## Images

Image panes support PNG, JPEG, GIF and WebP. The image fits inside the pane.
See [Panes](Panes) for ways to open one.

1. Drag a region at least 4 × 4 source pixels.
2. Click the quote icon. See [Quotes](Quotes) for the quote menu.

Click **Quote image** in the helper bar to quote the whole image.
You can also select the image pane and press `q` in canvas mode.
The image helper bar also has **Connect** and **Close**.

## PDFs

![A PDF pane with highlighted text](images/images-and-pdfs-1.png)

1. Click **Find** in the helper bar.
2. Click the PDF body to give it the keyboard.
3. Type your query.

To highlight and quote text:

1. Click **Highlight text** in the helper bar.
2. Drag across the text.
3. Click the quote icon.

Click a saved highlight to show its quote icon again.
See [Quotes](Quotes) for the quote menu.

- Click **Previous page** or **Next page** to move between pages.
- Click **Clear highlights** to remove all marks. The button shows when the pane has marks.
- The PDF helper bar also has **Connect** and **Close**.

Canopy saves marks in `canvas.json`, never in the PDF.
See [Agent Pane Commands](Agent-Pane-Commands) for what `canopy pane read` returns for images and PDFs.

Select the PDF pane in canvas mode to use these keys:

| Key | Action |
|---|---|
| `/` | Toggle **Find**. |
| `m` | Toggle **Highlight text**. |
| `[` | Go to the previous page. |
| `]` | Go to the next page. |
| `Shift+M` | Clear all highlights. |

## Troubleshooting

- **A region has no quote icon.** Drag a region at least 4 × 4 source pixels.
- **Typing does not search.** Click **Find**, then click the PDF body before typing.
- **"The PDF is password protected".** Canopy reports this in an error toast.

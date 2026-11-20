# Spaces

A space is a room for one project. It has a folder, a name, a color and its own canvas, and every terminal or agent you start in it opens in its folder. Most people keep one space per repository, plus extra spaces for worktrees or side experiments.

Home shows all your spaces at once, so you can see where work is happening and jump straight to the agent that needs you.

![Home with three space cards](images/spaces-1.png)

## Create a space

1. Click **+** in the top strip, or press `Cmd+N` (`Ctrl+Shift+N` on Linux and Windows). The **New space** dialog opens.
2. If you have used folders before, Canopy lists them. Click one to open a space there right away, or click **Open space** for the full form.
3. In the form, type a **Name**, then pick a folder with **Folder** > **Add** > **Choose**.
4. Click **Open space**.

Leave out the name and the space takes the folder's name. Leave out the folder and the space opens in your home folder. Several spaces can share a folder, which is handy when you want two separate canvases for the same project.

To create a space on another computer, see [Machines](Machines).

## Home and space cards

Home is the zoomed-out view of every space, each drawn as a card. A card shows the space's name and color, how many terminals it has, and how many agents are working in it. A bell on a card means an agent in that space needs you.

- Double-click a card to go into its space. Or select the card and press `Enter`.
- Click **Home** in the top strip, or press `Cmd+Shift+H` (`Ctrl+Shift+H` on Linux and Windows), to come back.
- Right-click a card to rename it, change its color, arrange its panes or delete it.

## Move between spaces

Once you have a few spaces, the keyboard is fastest:

- `Cmd+1` through `Cmd+9` (`Ctrl+1` through `Ctrl+9` on Linux and Windows) go to spaces in tab order.
- `Ctrl+Tab` returns to the previous space. Press it again to switch back, like flipping between two apps.
- `Ctrl+Shift+Tab` goes further back through spaces you visited.
- `Cmd+K` opens **Go to**, where you can type part of a space's name. See [Go To and Palette](Go-To-and-Palette).

Each space and Home keep their own pan and zoom, so you come back to the view you left.

## The top strip

The bar across the top of the window, from left to right:

- **Home** opens Home. It cannot be closed.
- **Space tabs**, one per space, with a folder icon and the name. A dot marks a space with an agent working or waiting for you. The active tab, and any tab you hover, shows a **Close space** button.
- **+** opens **New space**.
- The **grid** button turns on [tile view](Canvas#tile-view).
- The **folder** button, **Files**, opens or closes the [Files panel](Files-Panel).

## Rename, recolor or delete a space

- Double-click a card's name on Home to rename it. Press `Enter` or click elsewhere to save, or `Esc` to cancel.
- Right-click a card for **Rename**, **Color** or **Delete space**.

> **Warning:** Deleting a space ends all of its terminals and agents. You cannot undo it.

Select **Delete space** (or click **Close space** on its tab), then click **Delete** in the confirmation dialog.

## Worktree spaces

A worktree space is a space for a separate Git checkout of the same repository, on its own branch. Use one when you want agents to work on two branches at once without touching each other's files. The [Parallel Branches](Parallel-Branches) guide shows the whole workflow.

The worktree commands are in the spaces list of the [Classic layout](Classic-Layout)'s sidebar, which groups each worktree space under its main checkout.

**Create one:**

1. Right-click a space whose folder is in a Git repository.
2. Select **New worktree** and type a branch name.
3. Press `Enter`.

Canopy uses the branch if it exists, or creates it from the current checkout. It puts the checkout under `~/.herdr/worktrees/<repo>/` and opens a space for it.

**Open an existing one:** select **Open worktree** from the same menu, type to filter, pick one with the arrow keys and press `Enter`. Canopy goes to its space, or creates one. Press `Esc` to close the dialog.

**Remove one:**

> **Warning:** **Remove worktree** deletes the checkout folder and ends the space's terminals. Uncommitted changes and unpushed submodule commits are lost.

Select **Remove worktree** from a worktree space's menu and press `Enter` to confirm. The branch stays in the repository. The main checkout cannot be removed this way.

## See also

- [Canvas](Canvas): moving around inside a space.
- [Machines](Machines): spaces on other computers.
- [Concepts](Concepts#spaces-and-home): how spaces fit with the rest of Canopy.

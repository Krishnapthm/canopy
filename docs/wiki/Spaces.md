# Spaces

A space is a room for one project folder, with its own canvas, name and color.
Home shows your spaces as cards.

![Home with three space cards](images/spaces-1.png)

## Create a space

1. Click **+** in the top strip (**New space**).
2. Select a recent folder, or click **Open space** for the full form.
3. Type a **Name**.
4. To select a folder, use **Folder** > **Add** > **Choose**.
5. Click **Open space**.

The full form opens directly when there are no recent folders.
Without a name, the space uses the folder's name. Without a folder, it opens at your home folder.

Several spaces can use the same folder. A new space opens immediately.

For spaces on other computers, see [Machines](Machines).

## Move between spaces

Each Home card shows its name, color, terminal count and working agent count.
A bell means an agent in that space needs you.

- Double-click a card to enter its space. Or select the card and press `Enter`.
- Click **Home** in the top strip to return to Home.
- Click a space tab to enter that space.
- Press `Cmd+1` through `Cmd+9` (`Ctrl+1` through `Ctrl+9` on Linux and Windows) for spaces in tab order.
- Press `Ctrl+Tab` to return to the previous space. Press it again to switch back.
- Press `Ctrl+Shift+Tab` to visit older spaces.

Each space and Home keep their own pan and zoom.

## Rename, recolor or delete

- Double-click a card's name to rename it. Press `Enter` or click elsewhere to save. Press `Esc` to cancel.
- Right-click a Home card for **Rename**, **Color** or **Delete space**.

> **Warning:** Deleting a space ends its terminals. You cannot undo it.

Select **Delete space**, then click **Delete** in the confirmation dialog.

## Worktrees

Worktree spaces use separate Git checkouts. Manage them from the spaces list in the **Classic** layout's sidebar. The list groups them under the main checkout's space when it is present.

1. Right-click a space in the spaces list whose folder is in a Git repository.
2. Select **New worktree**.
3. Type a branch name.
4. Press `Enter` to create the checkout and open its space.

Canopy uses an existing branch or creates a branch from the current checkout.
Canopy creates checkouts under `~/.herdr/worktrees/<repo>/`.

Select **Open worktree** from the same menu to list existing checkouts.
Type to filter, select with the arrow keys, then press `Enter`.

Canopy enters an existing space for that checkout or creates one. Press `Esc` to close the dialog.

> **Warning:** **Remove worktree** deletes the checkout and ends the space's terminals. Uncommitted changes and unpushed submodule commits can be lost.

Select **Remove worktree** from a worktree space's menu, then press `Enter` to confirm. The branch stays.
The main checkout cannot be removed with this dialog.

## The top strip

- **Home** opens Home and has no close button.
- Space tabs show a folder icon and name. An accent dot marks a space with a working or blocked agent.
- The active or hovered tab shows **Close space**. It uses the same deletion confirmation.
- **+** opens **New space**.
- The grid button opens [Tile view](Canvas#tile-view).
- The folder button, **Files**, opens or closes the [Files panel](Files-Panel).

For focus fit, see [Canvas](Canvas#focus-fit).

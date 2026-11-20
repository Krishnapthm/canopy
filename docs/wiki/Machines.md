# Machines

A machine is another computer, such as a server, a cloud VM or a Mac mini, whose terminals and agents Canopy shows in the same window as your local ones. Canopy reaches it over SSH, and the machine runs its own herdr-core.

That last part is what makes machines useful: the agents run on the machine, not on your laptop. Start a long task on a server, close your laptop, and the agents keep working. Open Canopy again and they are where you left them. Machines are also a way to give agents more CPU, a different OS, or access to a network your laptop is not on.

## Add a machine

Install [herdr-core](herdr-core) on the remote computer first.

1. Click **Add a machine** in the welcome, or **Add machine** at the bottom of the **Classic** layout's sidebar.
2. Click **Add** beside a host from `~/.ssh/config`.

For a host outside the list, type its SSH address under **Or type how ssh reaches it**.
Click **Add**.

You can also add a machine from a terminal:

```sh
hc machine add user@host
```

## Use a machine

Once a machine is added, you create spaces on it, and everything in those spaces (terminals, agents, helpers) runs there.

- In the [Classic layout](Classic-Layout), the sidebar groups spaces by machine. Click **+** beside a connected machine to create a space there.
- In the canvas layout, the **New space** dialog creates local spaces only. To create a space on a machine from there, run this in any Canopy terminal (the machine name is the one you added):

  ```sh
  canopy do space.new --machine vps --folder /home/me/myapp
  ```

- Remote spaces appear on Home and in the top strip like any other space. See [Spaces](Spaces).
- Terminal addresses include the machine name: `mac/t7` is terminal `t7` on machine `mac`.

## Troubleshooting

- **Adding the machine needs a password or a host key answer.** The dialog tells you to run `hc machine add <host>` in a terminal.
  Run that command to answer the prompts.
- **The remote needs herdr-core.** Follow [herdr-core](herdr-core) for installation. The dialog gives the same terminal command when the core is missing or incompatible.
- **A card says "`name` is a Mac: allow its folders once".** macOS asks on that Mac's screen before an agent reads Desktop, Documents or Downloads.
  Click **Open the setting on `name`**. Press **+** and add the herdr-core file the card shows, to give it Full Disk Access.
- **The dialog says agents stop when you disconnect.** The server ends your programs when your last login closes. Run the `sudo loginctl enable-linger` command from the dialog on that machine once.

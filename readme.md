## noita-utility-box
[![Release](https://img.shields.io/github/v/release/necauqua/noita-utility-box)](https://github.com/necauqua/noita-utility-box/releases/latest)
[![CI](https://github.com/necauqua/noita-utility-box/actions/workflows/ci.yml/badge.svg)](https://github.com/necauqua/noita-utility-box/actions/workflows/ci.yml)
![License](https://img.shields.io/github/license/necauqua/noita-utility-box)
[![discord link](https://img.shields.io/discord/1346986932244054016)](https://discord.gg/RDdRT8Z8j9)

This is a memory reading tool that reads some useful data
directly from a running instance of [Noita](https://noitagame.com).
Anything that's considered true cheating is hidden by default and requires you
to enable it with checkboxes, collapsing panels and/or settings.

This is useful if you want **no** mods to be installed yet want to do some
advanced stuff.

Download from GitHub Releases [here](https://github.com/necauqua/noita-utility-box/releases).

### Tools
#### Orb Radar
It has an orb radar tool which completely automatically finds 34th orb GTG
locations for the running game and tracks player coords in realtime.

> [!NOTE]
> Click the image below to see a video of it in action, GitHub doesn't allow
> embedding mp4s and a gif of good enough quality would be too big.

[![Orb Radar demo](https://necauq.ua/images/orb-radar-demo.png)](https://necauq.ua/videos/orb-radar-demo.mp4)

#### Live Stats
Automatically gets current death/win/streak/best-streak counts, formats them
and sets an OBS text input (through obs-websocket which you can enable in OBS
menus)

#### Player Info
Shows several pieces of information about the player.
- Currently held wands - this is mostly useful for the `Wand Simulator` button
  that will automatically open the wand simulator for the given wand and its spells.
- Shows exact material IDs and amounts in player inventory flasks and puches.
- Exact floating point values for player HP and max HP, useful for knowing
  sub-integer and otherwise irregular HP values.
- Current player damage multipliers, similar to orb radar to avoid having to
  shut down the game and read `player.xml` if you want to know them.

#### .. more coming
There are plans for more stuff to come, some low handing fruits like hitless
checker, less low ones like modless streamer wands, I want to do a git backup
manager, a mod update manager, etc etc

##### Orb Radar
There is much room for improvements, being functionality or readability:

- [x] Display current world orb rooms in NG & NG+
- [x] Filter orbs that are already taken
- [ ] Better accessibility features for the radar

##### Ingame overlay
For people like @Larandar that don't have a second screen, inject in one way
of another on top of fullscreen Noita some information:

- [ ] Orb radar
- [ ] Coords
- [ ] Arbitrary entity component informations?

## Development

### Building from sources

`cargo build --release` will work to build the binary for your platform, but
for cross-compilation the thing that works reliably for me is Nix.

If you have [Nix](https://nixos.org/download/) set up, you can build the flake
apps `.#windows`, `.#linux` and `.#deb`:

- `nix build . .#{{target}}`

The `justfile` provides the following shortcuts (given you have `just` installed):
- `just check` will check for formatting and clippy errors to save on waiting for CI to catch those
- `just build (windows|linux|deb)` to build a specific artifact (still requires Nix, just an alias for the above)
- `just build-all` to build all three

## License
It's MIT, please have a copy of the LICENSE file in your derivatives so that my
name is there lol

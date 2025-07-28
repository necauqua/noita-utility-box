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
- [ ] Arbitrary entity component information?

### Development

`cargo build/run` should just work to build or run the binary for your
platform, given that you have [Rust](https://rustup.rs) installed.

If you're not on Windows, you can build the Windows .exe by doing:
  - `rustup add x86_64-pc-windows-gnu` once and having `mingw-w64` installed on your system.
  - And then `cargo build --release --target x86_64-pc-windows-gnu`

If you're on NixOS (or just use Nix) the dev shell in `flake.nix` provides all
necessary dependencies for the above commands to work, you can enter is with
`nix develop` but I suggest using `direnv`.

To get a bit-by-bit reproducible builds of artifacts downloadable from
[releases](https://github.com/necauqua/noita-utility-box/releases), you can set
up [Nix](https://nixos.org/download/), and use
```bash
nix build .#linux
# or
nix build .#deb
# or
nix build .#windows
# or all at once
nix build .#linux .#deb .#windows
```

On NixOS the flake app is installable with
```bash
nix profile install github:necauqua/noita-utility-box
```
to get a bleeding edge version (from latest `main` commit) or
```bash
nix profile install github:necauqua/noita-utility-box/release
```
for the latest release.

If have `just` installed, there's a convenient `just check` shortcut that will
check for formatting and clippy errors to save on waiting for CI to catch
those (it will probably not work well on Windows).

### Contribution
If you want to contribute, feel free to open a PR here on github, or send
patches on [tangled](https://tangled.sh/@necauq.ua/noita-utility-box),
[radicle](https://app.radicle.xyz/nodes/iris.radicle.xyz/rad%3Az2n8gDK7BUhNrt2aV2wCanazHoSSN)
or even to my (e-mail)[mailto:him@necauq.ua?subject=noita-utility-box].

- This repo follows a convention of small atomic commits (the smaller the
  better) that I will fast-forward into the `main` branch once you pass my
  reviews. This means your PGP signatures and commit authorship will be fully
  preserved in git history.

- Since the commit (before that I was a bit loose) introducing this contribution
  section, this repo strictly follows the
  [conventional commits](https://www.conventionalcommits.org/)
  naming scheme, I *will* ask you to reword commits.
  Also I find this
  [cheat sheet](https://gist.github.com/qoomon/5dfcdf8eec66a051ecd85625518cfd13)
  to be easier to digest, take a look.

- Any review comment fixes **must** be amended into respective commits, the
  final patchset should be clear of any "addressing reviews" commits.
  You can read
  [this](https://gist.github.com/thoughtpolice/9c45287550a56b2047c6311fbadebed2)
  excellent writeup on the topic. Also I cannot recommend enough checking out
  [jujutsu](https://github.com/jj-vcs/jj#readme) which makes all of the above
  that much easier compared to git.

### License
It's MIT, please have a copy of the LICENSE file in your derivatives so that my
name is there lol

## noita-utility-box
[![Release](https://img.shields.io/github/v/release/necauqua/noita-utility-box)](https://github.com/necauqua/noita-utility-box/releases/latest)
[![CI](https://github.com/necauqua/noita-utility-box/actions/workflows/ci.yml/badge.svg)](https://github.com/necauqua/noita-utility-box/actions/workflows/ci.yml)
![License](https://img.shields.io/github/license/necauqua/noita-utility-box)
[![discord link](https://img.shields.io/discord/587713408841940993)](https://discord.com/invite/pZrbJNb)

This is a cheatengine-style memory reader that reads useful data
directly from a running instance of [Noita](https://noitagame.com).

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

#### Material Pipette
Shows exact material IDs and amounts in player inventory flasks.
This is both for Furys alchemy run and to implement and test the ability to
read entity components - modless (readonly) component explorer and streamer
wands are within my grasp.

#### .. more coming
There are plans for more stuff to come, some low handing fruits like hitless
checker, less low ones like modless streamer wands, I want to do a git backup
manager, a mod update manager, etc etc

## License
It's MIT, please have a copy of the LICENSE file in your derivatives so that my
name is there lol

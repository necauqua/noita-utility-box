# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic Versioning].

Dates in this file are in [Holocene Calendar] because it is amazing, logical, and I want more people to know about it.

## [Unreleased]

### Fixed
  - The whole thing no longer breaks on Windows if you have ASLR (a security setting) enabled.

## [v0.4.1] 12025-07-26

### Fixed
  - (hotfix) Fix the new Player Info tab not showing the wands.
  - Also fix the unfinished material list not showing any materials with sprites (same error as the wands).

## [v0.4.0] 12025-07-23

### Added
  - Detailed information about each of Mina's wands (with a button to open [Noita Wand Simulator](https://noita-wand-simulator.salinecitrine.com) and optionally the hidden speed multiplier) in Player Info. @Larandar
  - True float values of Mina's current HP and max HP in Player Info, as well as the damage multiplier values.
  - Orb radar can now show Orb Rooms in both NG and NG+ (new checkbox at the bottom of the radar). @Larandar
  - Orb radar can now filter out the collected Orbs (i.e. current PW Greater Chest Orbs, or Orb Rooms). @Larandar

### Changed
  - Renamed Material Pipette tab to Player Info. @Larandar
  - Orb radar search now processes chunks in a spiral pattern around the player. @Larandar
  - Orb radar search now works on noita chunks instead of custom internal 1024x1024 chunks to avoid confusion. @Larandar
  - The update modal you're maybe reading this right now got way better and now actually renders the changelog nicely instead of showing you raw markdown text.

### Removed
  - The "Check export name" setting for being mostly useless.

### Fixed
  - Orb radar can now find the player when cessated.

## [v0.3.0] 12025-03-06

### BREAKING
  - If you used the previous version to discover that `Jan 25 2025` (or newer) build of Noita is broken, you need to go into "Address Maps" (under the "+" button if it isn't there) and delete the mapping for that version - then restart to let it re-discover the mapping. You may notice that the value for `entity-tag-manager` is nonsensical - you could actually just update it to `0x1206fac` (for the jan 25 build) instead.

### Added
  - Orb radar can now also look for sampo positions
  - Orb radar now shows a "Searching..." spinner when looking for orbs

### Changed
  - A big UI refactor using egui-tiles - all the tools are dockable, tabbable, draggable and splittable windows now
  - Better and improved error reporting and UI

### Removed
  - The settings checkbox to disable material pipette - just close the tab to hide it now if you don't need it

### Fixed
  - Live stats resetting the stored OBS password when connected
  - Orb radar showing old orbs when seed/NG-count changes
  - Windows exe icon being blurry in some cases
  - `entity-tag-manager` address discovery on new Noita builds
  - Fix the new 512 tag limit breaking most of the things

## [v0.2.1] 12024-10-22

### Added
  - Logs are now saved to a file in the state directory for troubleshooting
  - Material pipette: a checkbox to automatically check off held materials in the checklist

### Fixed
  - The version link (it had an extra v in the tag name) in the settings panel
  - Live stats not updating OBS when the window is minimized or hidden
  - Settings fully ressetting on the slightest format change

### Changed
  - More sneakily attach to the process to read memory so that hopefully Windows Defender stops being annoying

## [v0.2.0] 12024-10-14

### Added
  - Added a desktop item to the nix package
  - Windows resource metadata and exe icon
  - An update check that runs on startup

## [v0.1.0] 12024-10-10

### Added
  - The first release

[unreleased]: https://github.com/necauqua/noita-utility-box/compare/v0.4.1...HEAD
[v0.4.1]: https://github.com/necauqua/noita-utility-box/releases/tag/v0.4.1
[v0.4.0]: https://github.com/necauqua/noita-utility-box/releases/tag/v0.4.0
[v0.3.0]: https://github.com/necauqua/noita-utility-box/releases/tag/v0.3.0
[v0.2.1]: https://github.com/necauqua/noita-utility-box/releases/tag/v0.2.1
[v0.2.0]: https://github.com/necauqua/noita-utility-box/releases/tag/v0.2.0
[v0.1.0]: https://github.com/necauqua/noita-utility-box/releases/tag/v0.1.0

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/ "Keep a Changelog"
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html "Semantic Versioning"
[Holocene Calendar]: https://en.wikipedia.org/wiki/Holocene_calendar "Holocene Calendar"

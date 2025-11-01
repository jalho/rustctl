/*
 * $ typst --version
 * typst 0.14.0 (unknown hash)
 *
 * $ typst compile ./source-file.typ ./target-file.pdf
 */

#let global_block_fill  = rgb(50,50,50)
#let global_block_inset = 8pt

#set document(
  title: [Installing SteamCMD on Debian]
)
#set page(fill: rgb(20, 20, 20))
#set text(fill: rgb(230, 230, 230))
#set heading(numbering: "1.1")

#title()

2025-11-01

= Overview

#table(
  columns: 4,
  [*component*], [*name*],           [*version*],             [*notes*],
  [OS],          [Debian GNU/Linux], [`13 (trixie)`],         [latest as of 2025-11-01],
  [kernel],      [Linux],            [`6.12.48+deb13-amd64`], [],
  [],            [SteamCMD],         [`0~20180105-5`],        [package name: `steamcmd`#footnote[https://packages.debian.org/trixie/steamcmd (accessed 2025-11-01)], executable: `/usr/games/steamcmd`],
)

= Configure Architecture

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  # dpkg --add-architecture i386
  # apt update
  ```
]

= Configure Non-Free Area

Edit `/etc/apt/sources.list.d/debian.sources`, which looks like the following
by default:

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  Types: deb deb-src
  URIs: mirror+file:///etc/apt/mirrors/debian.list
  Suites: trixie trixie-updates trixie-backports
  Components: main
  Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg

  Types: deb deb-src
  URIs: mirror+file:///etc/apt/mirrors/debian-security.list
  Suites: trixie-security
  Components: main
  Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg
  ```
]

Add `non-free` to `Components`:

#block(fill: global_block_fill, inset: global_block_inset)[
  ```diff
  4c4
  < Components: main
  ---
  > Components: main non-free
  ```
]

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  # apt update
  ```
]

#pagebreak()

= Install SteamCMD

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  # apt install steamcmd
  ```
]

The installer is interactive: You'll need to accept some terms.

The package installs the program in a weird location: `/usr/games/steamcmd`,
which isn't even in the default `$PATH`. Create a link to the executable in a
more sensible location.

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  # ln -s /usr/games/steamcmd /usr/bin/steamcmd
  ```
]

SteamCMD updates itself outside of the system package manager. As of 2025-11-01,
the latest version seems to be `1759461699`, as reported by the program when run
with `--help`.

#block(fill: global_block_fill, inset: global_block_inset)[
  ```
  # steamcmd --help
  [  0%] Checking for available updates...
  [----] Verifying installation...
  Steam Console Client (c) Valve Corporation - version 1759461699
  ```
]

The version number `1759461699` translates to `2025-10-03 03:21:39 UTC`, if
interpreted as a Unix timestamp, which seems reasonable as of 2025-11-01.

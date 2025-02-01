# CHANGELOG.md

## 0.2.0-wip

**Work in progress!**

Some prominent characteristics compared to 0.1.0:

- Refactor: Rewrote the whole thing as a state machine, reorganized modules and
  redefined error cases
- Dropped feature: System calls' tracking: It was fun but added unnecessary
  complexity without much utility
- Dropped feature: Automatic installation of _SteamCMD_: Now expecting it
  to be pre-installed to simplify the program. Automatic installations of
  _RustDedicated_ and _Carbon_ are still in scope of the program.

## 0.1.0

Initial implementation of game bootstrapper.

Some prominent features:

- Automatic installation of _SteamCMD_
- Automatic installation of _RustDedicated_
- Automatic installation of _Carbon_
- Single file based configuration for the whole program
- Automatic updating of the game server before launching
- Automatic configuration of _Carbon_ during game server runtime: Categorize the
  instance as _not modded_
- System calls' tracking while installing or running software: Using `strace` to
  track changed files and network connections made

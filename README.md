Copyright (C) Jaume Delclòs (aka cosarara97) - 2014

RustGB
------

This is my WIP Game Boy emulator written in Rust.
Until I think of a proper name for it I'll call it rustgb.

Right now... Well, it doesn't work. This is the status of the project:

* The CPU is quite well implemented, but not enough.
It passes all of blargg's tests individually (yes!), but won't run the
all-in-one version properly (it restarts on test 5, IIRC).

* Tetris shows only the first screen (copyright and stuff),
bomberman shows it's first screen as well, Pokémon almost gets it,
and Super Mario Land loads the tileset but not the tile map.

* That demo ROM displaying a fish's picture and a text editor almost works,
although there is a small glitch in the picture's frame.

* Only tilemaps are implemented (no sprites). There is no sound either.

* It is too slow. It won't be until it runs at 60fps in my crappy Intel
Atom netbook, which doesn't at the moment. Tweeking the input poll
rate makes it faster.

Building
--------

Install rust and cargo nightlies and run *cargo build*.
You'll find the binary in the target/ directory.

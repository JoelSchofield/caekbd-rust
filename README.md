# caekbd-rust

## Debuging with openocd

Need to be running the openocd server, built with support for the picoprobe (not currently in main openocd branch). After building, run with:

```bash
openocd -f interface/picoprobe.cfg -f target/rp2040.cfg -s tcl
```

You can now start a debug session in vscode. Setting breakpoints does not currently seem accurate...

## Building

```bash
cargo build
```

or

```bash
cargo build --release
```

## Flashing

Put the RPi into UF2 mode. If the keyboard software is running this is done by `Fish+DEL`.

If successful you will see the RPi mount as a drive like a flash stick. 

Then:

```bash
cargo run --release
```

This will flash the firmware to the Pico.

Done!
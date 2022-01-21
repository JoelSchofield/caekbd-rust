# caekbd-rust

Need to be running the openocd server, built with support for the picoprobe (not currently in main openocd branch). After building, run with:

``` bash
openocd -f interface/picoprobe.cfg -f target/rp2040.cfg -s tcl
```

You can now start a debug session in vscode. Setting breakpoints does not currently seem accurate...

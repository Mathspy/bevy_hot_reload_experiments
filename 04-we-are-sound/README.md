# Sound hot reloading!

This experiment demonstrates that we don't have to rely on the unstable extern
"Rust" ABI for our hot reloading, which is "technically" unsound.

Instead we expose a bunch of static extern "C" functions that return and accept
opaque pointers and allow the binary to interface with the dylib without needing
to rely on any of its actual struct representations.

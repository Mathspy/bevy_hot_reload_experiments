# Hot reload by wrapping Bevy app

This is a demonstration of hot reloading a Bevy app by wrapping the app with a
binary that runs the app and detects when it needs to be reloaded and reloads
it.

This depends on some minimal updates to Bevy which can be found here:
https://github.com/Mathspy/bevy/tree/hot-reload-3

A lot of the guts are currently handled in the library because of type id
mismatches. This is because we compile the binary completely independently from
the dylib. If they were in the same workspace I believe we could move most(all?)
of the guts into the binary wrapper and then this could truly be a simple
non-invasive wrapper

Most of the interesting logic lives in `hot_reload_app` which currently swaps
out the runner from the new app and the schedules. The schedules are the main
goal, because we wanna run with the new set of schedules, the runner is
necessary because .run() currently "loses" the runner (mem::replace shenangins),
this could technically be fixed by just putting the runner in its place after a
run exits.

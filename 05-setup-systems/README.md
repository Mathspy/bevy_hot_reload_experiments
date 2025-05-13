# Setup systems aren't reloaded

Setup systems will only ever run once while doing hot reloads. They will be run
again if we do cold reload and reset the state from scratch but otherwise they
shouldn't need to rerun.

The way we handle this is very simple, internally `Main` schedule has a single
system which is basically the scheduling system that runs the other systems.
That system has a `Local` which determines whether the setup systems were ran or
not yet, and since `Main` is an internal schedule that people shouldn't be
adding systems too (at least if they do it's not clear or documented when those
systems will run, or if they ever will) that will never need to be reloaded,
since it's fixed inside of Bevy's code instead of user code, we add it to the
reconciliation step (we perserve it from the old `World` in the new new
`Schedules`)

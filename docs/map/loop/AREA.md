+++
title = "Update loop"
band = "loop"
summary = "Pull, build, restart the tool, and stop everything cleanly"
+++

The event loop in `src/main.rs` (`App`): one thread owns all state; pulls,
child output and child exits arrive on a channel.

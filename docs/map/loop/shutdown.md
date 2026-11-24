+++
title = "Shutdown"
summary = "Ctrl-C, q, SIGINT, SIGTERM, SIGHUP: stop the build and the tool, wait, restore the terminal"
aka = ["Ctrl-C", "quit", "exit 130"]
kind = "code"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["src/main.rs"]

[[feature]]
id = "graceful"
title = "Stop pulling, stop both children with the R5 rules, exit when both are gone (R6)"
status = "done"

[[feature]]
id = "second"
title = "A second Ctrl-C SIGKILLs everything without waiting"
status = "done"

[[feature]]
id = "signals"
title = "SIGINT, SIGTERM and SIGHUP do the same as Ctrl-C; exit code 130"
status = "done"

[[feature]]
id = "terminal"
title = "The terminal is restored, also on panic"
status = "done"
milestone = "m2"

[[needs]]
on = "/loop/procs#group-signals"
why = "Stopping means signalling each child's group"

[[needs]]
on = "/deps/signal-hook"
why = "Turns signals into events on the main loop"
+++

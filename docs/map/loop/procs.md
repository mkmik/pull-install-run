+++
title = "Child processes"
summary = "sh -c CMD in its own pty and session, so the shell and its children are one process group"
aka = ["pty", "setsid", "killpg", "process group"]
kind = "code"
intent = "induced"
research = "done"
milestone = "m1"
refs = ["src/main.rs"]

[[feature]]
id = "pty"
title = "Each child gets a pseudo-terminal and its own session"
status = "done"

[[feature]]
id = "group-signals"
title = "Signals go to the whole group; leftovers are SIGKILLed when the leader exits"
status = "done"

[[feature]]
id = "ordered-exit"
title = "The exit is reported after the last output, waiting at most 500 ms for the pty to drain"
status = "done"

[[feature]]
id = "pdeathsig"
title = "On Linux, children die if pir is SIGKILLed (PR_SET_PDEATHSIG)"
status = "todo"
note = "portable-pty doesn't expose pre_exec; for now the tool's pid is logged when it starts"

[[needs]]
on = "/deps/portable-pty"
why = "Opens the pty and spawns the child with setsid"
+++

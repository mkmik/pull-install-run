+++
title = "Tool supervision"
summary = "Runs --run; replaces it after a green build and never after a red one"
aka = ["restart", "run command"]
kind = "code"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["src/main.rs"]

[[feature]]
id = "green-only"
title = "Old tool keeps running during the build; restart only after exit 0 (R3)"
status = "done"

[[feature]]
id = "stop"
title = "Stop = SIGTERM to the group, SIGKILL after --stop-timeout, then start the new one (R5)"
status = "done"

[[feature]]
id = "exit"
title = "A tool that exits by itself is not restarted; status shows the exit code (R7)"
status = "done"

[[feature]]
id = "manual"
title = "r restarts the tool"
status = "done"

[[needs]]
on = "/loop/procs"
why = "Start, signal and reap the tool's process group"
+++

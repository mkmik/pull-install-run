+++
title = "Plain mode"
summary = "No split screen: every line printed with a [build], [run] or [pir] prefix"
aka = ["--plain", "non-tty"]
kind = "code"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["src/main.rs"]

[[feature]]
id = "prefixed"
title = "Line-prefixed output; a progress bar's \\r keeps only its last state (R9)"
status = "done"

[[feature]]
id = "auto"
title = "On by itself when stdout is not a terminal"
status = "done"

[[feature]]
id = "pir-prefix"
title = "pir's own events prefixed [pir], not [build]"
status = "done"
note = "Differs from the PRD, which put them under [build]"
+++

+++
title = "Build"
summary = "Runs --build when HEAD differs from the last commit built; one build at a time"
aka = ["go install", "cargo install"]
kind = "code"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["src/main.rs"]

[[feature]]
id = "on-change"
title = "Build only when HEAD changed, including by a manual pull; failures wait for a new commit (R2)"
status = "done"

[[feature]]
id = "serial"
title = "Never two builds at once; a commit during a build queues one more (R4)"
status = "done"

[[feature]]
id = "force"
title = "b builds now, without a new commit"
status = "done"

[[feature]]
id = "first-build"
title = "If the first build fails, start the previously installed tool anyway"
status = "done"

[[needs]]
on = "/loop/procs"
why = "The build runs as a child in its own pty and process group"

[[needs]]
on = "/loop/run"
why = "A green build restarts the tool"
+++

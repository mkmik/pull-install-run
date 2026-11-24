+++
title = "Pull"
summary = "Every --every: git fetch, then merge --ff-only @{u}; never resets, stashes or merges"
aka = ["git pull", "fetch", "fast-forward"]
kind = "code"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["src/main.rs"]

[[feature]]
id = "timer"
title = "Pull at start and every --every; p pulls now (R1)"
status = "done"

[[feature]]
id = "retry"
title = "A failed fetch is retried 3 times, 10 s apart (R1)"
status = "done"

[[feature]]
id = "ff-only"
title = "Can't fast-forward (local commits, no upstream, detached HEAD): log and skip (R8)"
status = "done"

[[feature]]
id = "quiet-repeats"
title = "The same warning on every pull is logged once"
status = "done"

[[feature]]
id = "commit-log"
title = "Log the pulled range and its commit subjects (R13)"
status = "done"
milestone = "m3"

[[needs]]
on = "/deps/git"
why = "Runs git fetch, merge, rev-parse and log"

[[needs]]
on = "/loop/build"
why = "A new HEAD starts a build"
+++

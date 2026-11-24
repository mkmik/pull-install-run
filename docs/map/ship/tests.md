+++
title = "Tests"
summary = "Unit tests, and an end-to-end script against a throwaway git repo"
aka = ["cargo test", "tests/e2e.sh"]
kind = "infra"
intent = "goal"
research = "done"
refs = ["tests/e2e.sh", "src/main.rs"]

[[feature]]
id = "unit"
title = "cargo test: duration parsing, line splitting"
status = "done"

[[feature]]
id = "e2e"
title = "e2e.sh: good commit restarts, broken commit doesn't, Ctrl-C leaves no processes, log files"
status = "done"
needs = ["/use/plain", "/use/cli#log-dir", "/loop/shutdown"]

[[feature]]
id = "tui"
title = "Drive the split screen in a pty and check what it draws"
status = "todo"
note = "Done by hand with a Python pty and pyte; not in the repo"
needs = ["/use/screen"]
+++

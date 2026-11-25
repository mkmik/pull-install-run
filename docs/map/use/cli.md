+++
title = "pir command"
summary = "pir [--build CMD] [--run CMD] [--every 1m] [--stop-timeout 10s] [-C DIR] [--log-dir DIR] [--plain]"
aka = ["flags", "clap", "Args"]
kind = "code"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["src/main.rs", "README.md", "https://claude.ai/artifact/XBEngRi9Lza4uFqwAVkmgv"]

[[feature]]
id = "flags"
title = "--build, --run, --every, --stop-timeout, -C"
status = "done"

[[feature]]
id = "optional-run"
title = "--run is optional; without it pir only pulls and builds"
status = "done"

[[feature]]
id = "autodetect"
title = "--build is optional; guessed as just install, just build, go install, then cargo install --path ."
status = "done"
note = "The justfile recipes come from `just --summary`, so `just` must be installed"

[[feature]]
id = "durations"
title = "Durations as 30s, 1m, 1h or bare seconds"
status = "done"

[[feature]]
id = "log-dir"
title = "--log-dir: one timestamped file per build and per run (R12)"
status = "done"
milestone = "m3"

[[needs]]
on = "/deps/clap"
why = "Parses the flags"

[[needs]]
on = "/loop/pull"
why = "The loop the command starts"

[[needs]]
on = "/loop/run"
why = "Keeps the tool running"

[[needs]]
on = "/loop/shutdown"
why = "Ctrl-C, SIGTERM and SIGHUP"
+++

The PRD is the "pull-install-run (pir) — PRD" doc linked in `refs`.

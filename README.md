# pull-install-run (`pir`)

`pir` keeps a tool you are developing up to date while you use it.

Every minute it pulls the current branch. When a new commit arrives it runs your build command. If the build succeeds, it stops the running tool and starts the new one. If the build fails, the old tool keeps running and the error stays on screen.

```sh
cargo install --path .

pir --build 'go install ./cmd/foo' --run 'foo serve --port 8080'
pir --build 'cargo install --path .' --run 'bar watch' --every 30s
pir --build 'make install' --run './run-dev.sh' --plain | tee pir.log
```

| Flag | Default | Meaning |
| --- | --- | --- |
| `--build` | required | Shell command that compiles and installs. Exit code 0 means success. |
| `--run` | required | Shell command that runs the tool. |
| `--every` | `1m` | Time between pulls (`30s`, `5m`, `1h`). |
| `--stop-timeout` | `10s` | How long to wait after SIGTERM before SIGKILL. |
| `--plain` | off | No split screen. Each line is prefixed with `[build]`, `[run]` or `[pir]`. Turns on by itself when stdout is not a terminal. |
| `-C <dir>` | `.` | Repo directory. |
| `--log-dir <dir>` | none | Also write every build and every run to its own timestamped file. |

## Screen

The left column shows build output and `pir`'s own events: what was pulled, build start and result, restarts. The right column shows the tool's output. Both columns are small terminal emulators, so colors and progress bars look the same as in a normal shell. The bottom line shows the branch, commit, build result, tool state and time to the next pull.

| Key | Action |
| --- | --- |
| `p` | Pull now |
| `b` | Build now, even without a new commit |
| `r` | Restart the tool |
| `Tab` | Switch which column has focus (yellow border) |
| `z` | Make the focused column full width, and back |
| `PgUp` `PgDn` `↑` `↓` `Home` `End` | Scroll the focused column (10,000 lines kept) |
| `q` | Quit |
| `Ctrl-C` | Quit with exit code 130. Press it again to kill everything without waiting. |

## Rules it follows

- **Pulling:** `git fetch`, then `git merge --ff-only @{u}`. If the fetch fails it retries 3 times, 10 seconds apart. If the branch can't be fast-forwarded (local commits, no upstream, detached HEAD), it logs that and does nothing else. It never resets, stashes or creates merge commits.
- **Building:** it builds when `HEAD` differs from the last commit it built, including when you pull by hand. A failed build is not retried until a new commit arrives (or you press `b`). Only one build runs at a time; a commit that lands during a build gets one more build afterwards.
- **First start:** it pulls and builds, then starts the tool. If that first build fails, it starts the previously installed tool anyway.
- **Tool exits by itself:** it is not restarted. The status bar shows the exit code. Press `r`, or the next successful build starts it.
- **Stopping:** each command runs as `/bin/sh -c '<cmd>'` in its own session and pseudo-terminal, so the shell and everything it started form one process group. Stopping means SIGTERM to that group, then SIGKILL after `--stop-timeout`. On quit, `pir` waits for both the build and the tool to be gone before it exits. SIGTERM and SIGHUP to `pir` work the same as Ctrl-C.
- **Input:** the tool gets no keyboard input. All keys go to `pir`.

## Known limits

- Scrolling uses keys only. Turning on mouse-wheel scrolling would stop you from selecting text with the mouse.
- Text already on screen is not re-wrapped when a column changes width (on `z` or a window resize).
- If `pir` itself is killed with SIGKILL, the tool keeps running. Its pid is logged when it starts.

## Tests and CI

```sh
cargo test                  # unit tests
tests/e2e.sh                # real git repo: good commit, broken commit, Ctrl-C cleanup
todography lint --strict    # the map in docs/map
```

`.github/workflows/ci.yml` runs fmt, clippy, the tests on Linux and macOS, and the map lint on every push and pull request.

## The map

`docs/map/` is a [todography](https://github.com/mkmik/todography) map of pir's parts: what each does, how far along it is, and what it needs. `todography next --all` lists the work that's left; `todography open` draws the map.

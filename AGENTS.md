# pir

`pir` pulls a git repo on a timer, rebuilds when the commit changes, and restarts a tool only after a green build. All code is in `src/main.rs`; usage is in `README.md`.

## Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
tests/e2e.sh          # real git repo, real processes; ~15 s
todography lint --strict
```

CI (`.github/workflows/ci.yml`) runs all of these, the tests on Linux and macOS.

## The map

`docs/map/` is pir's [todography](docs/map/README.md) map: one Markdown file per component, with TOML front matter. The format is at https://mkmik.github.io/todography/reference/format.html.

When you change code:

- When a feature lands, set its `status = "done"` in its map file, in the same commit.
- If you find something missing in another component, add a `[[feature]]` with `status = "todo"` there, and a need on it from the component you're working on.
- If you add a new piece because something needed it, add a component with `intent = "induced"` and the needs that pulled it in.
- Refer to components in comments and commit messages as `t8y:/area/component`.
- Run `todography lint --strict` before committing.

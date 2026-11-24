+++
title = "CI"
summary = "GitHub Actions: fmt, clippy, tests on Linux and macOS, and todography lint on every push and PR"
aka = ["github actions", "ci.yml"]
kind = "infra"
intent = "goal"
research = "done"
refs = [".github/workflows/ci.yml"]

[[feature]]
id = "lint"
title = "cargo fmt --check and cargo clippy -D warnings"
status = "done"

[[feature]]
id = "test"
title = "cargo test and tests/e2e.sh on ubuntu-latest and macos-latest"
status = "done"
needs = ["/ship/tests#e2e"]

[[feature]]
id = "map"
title = "todography lint --strict and build"
status = "done"
needs = ["/ship/map"]

[[feature]]
id = "pinned"
title = "Actions pinned to commit SHAs; read-only GITHUB_TOKEN"
status = "done"
+++

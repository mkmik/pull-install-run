+++
title = "Split screen"
summary = "Build output left, tool output right, each a terminal emulator; status bar and keys below"
aka = ["TUI", "panes", "status bar", "ratatui"]
kind = "code"
intent = "goal"
research = "done"
milestone = "m2"
refs = ["src/main.rs"]

[[feature]]
id = "panes"
title = "Two columns, each a vt100 emulator, so colors and progress bars render"
status = "done"

[[feature]]
id = "pty-size"
title = "Each child's pty sized to its column and resized with the window (R10)"
status = "done"

[[feature]]
id = "status-bar"
title = "Branch, commit, build result, tool state, countdown to next pull"
status = "done"

[[feature]]
id = "keys"
title = "p pull, b build, r restart, Tab focus, q and Ctrl-C quit"
status = "done"

[[feature]]
id = "scrollback"
title = "10,000 lines per column; PgUp, PgDn, arrows, Home, End (R11)"
status = "done"
milestone = "m3"

[[feature]]
id = "zoom"
title = "z makes the focused column full width (R14)"
status = "done"
milestone = "m3"

[[feature]]
id = "reflow"
title = "Re-wrap text already on screen when a column changes width"
status = "todo"
note = "vt100 doesn't reflow; it would mean keeping raw output and replaying it at the new width"

[[feature]]
id = "mouse-scroll"
title = "Scroll with the mouse wheel"
status = "dropped"
note = "Mouse capture would stop people from selecting text with the mouse"

[[needs]]
on = "/deps/ratatui"
why = "Layout, borders, status bar, raw mode and key events"

[[needs]]
on = "/deps/tui-term"
why = "Draws each child's output as a terminal would"

[[needs]]
on = "/loop/procs#pty"
why = "Children must see a terminal to emit colors"
+++

+++
title = "Split screen"
summary = "Build on top, tool below, each a terminal emulator; a thin draggable line between; status bar"
aka = ["TUI", "panes", "status bar", "ratatui"]
kind = "code"
intent = "goal"
research = "done"
milestone = "m2"
refs = ["src/main.rs"]

[[feature]]
id = "panes"
title = "Two halves, each a vt100 emulator, so colors and progress bars render"
status = "done"

[[feature]]
id = "pty-size"
title = "Each child's pty sized to its half and resized with the window (R10)"
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
title = "10,000 lines per half; PgUp, PgDn, arrows, Home, End (R11)"
status = "done"
milestone = "m3"

[[feature]]
id = "zoom"
title = "z makes the focused half fill the screen (R14)"
status = "done"
milestone = "m3"

[[feature]]
id = "reflow"
title = "Re-wrap text already on screen when a half changes width"
status = "todo"
note = "vt100 doesn't reflow; it would mean keeping raw output and replaying it at the new width"

[[feature]]
id = "drag-split"
title = "Borderless panes; drag the one-row divider with the mouse to resize, click a pane to focus it"
status = "done"
note = "Takes the mouse: selecting text needs Shift (Option on macOS terminals)"

[[feature]]
id = "mouse-scroll"
title = "Scroll with the mouse wheel"
status = "done"
note = "Needed once drag-split captured the mouse, since the terminal no longer scrolls"

[[needs]]
on = "/deps/ratatui"
why = "Layout, divider, status bar, raw mode, key and mouse events"

[[needs]]
on = "/deps/tui-term"
why = "Draws each child's output as a terminal would"

[[needs]]
on = "/loop/procs#pty"
why = "Children must see a terminal to emit colors"
+++

# The map

`docs/map/` is pir's [todography](https://github.com/mkmik/todography) map:
one Markdown file per part of pir, saying what it does, how far along it
is, and what it needs. The format is described at
https://mkmik.github.io/todography/reference/format.html.

```sh
cargo install todography --locked
todography lint --strict   # CI runs this
todography next --all      # the work that's left
todography open            # draw the map in a browser
todography show loop/run   # one component, what it needs and what needs it
```

Change the map in the same commit as the code: when a feature lands, set
its `status = "done"`.

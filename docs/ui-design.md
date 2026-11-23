# The visual surface

Treat the application as a drawing board beside a conversation, not a dashboard.
The rendering is the main content; its controls and provenance sit outside it.
Keyboard interaction is the primary path: a searchable switcher, pane shortcuts,
revision keys, comment submission, and in-app shortcut discovery. Publishing
preserves focus, and app shortcuts remain available inside isolated renderings.
Use a slate conversation rail, a white canvas on a faint drafting grid, and a
separate comment rail. Avoid decorative cards, hero copy, and unsolicited motion.

Tokens: paper `#ffffff`, ink `#25364a`, slate `#e7edf4`, line `#d4dfe9`,
blue `#275ea8`, muted `#66788a`. Noto Sans/Liberation Sans supplies quiet tool
typography without external font requests. Titles are left aligned; a rendering
chooses its own composition. Short workspace identifiers distinguish duplicate
conversation titles. Keyboard focus uses a visible blue outline.

At narrow widths the comment rail moves below the canvas; on small screens the
navigation becomes a horizontal strip. The explicit revision selector and draft
target carry meaning: live updates must not change what a person is discussing.

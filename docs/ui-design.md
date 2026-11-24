# The visual surface

## Brief and point of view

A dark, keyboard-first visual workbench for a programmer discussing ideas with
agents. The agent's rendering is the hero; this is a projection room and an image
review instrument, not a dashboard or landing page. The app should recede while
keeping provenance, revision identity, and feedback within immediate reach.

## Design pass: compact tokens

| Role | Color |
| --- | --- |
| Projection stage | `#181923` |
| Conversation index | `#20212c` |
| Controls and selection | `#2a2c39` |
| Main text | `#e7e5e0` |
| Quiet text | `#aaa8b7` |
| Keyboard focus | `#b8b2dc` |

Borders use a derived `#3c3e50`; errors have their own semantic rose colors. The
focus lavender is restrained and functional rather than a single neon accent.

Cantarell is the UI face, available locally on this desktop, with Noto Sans and
Liberation Sans fallbacks. Use 14–15px body text, 12px contextual controls, and a
21px medium-weight visual title. Keep headings left aligned and subordinate to
the rendering. Monospace belongs to actual key chords and code, not metadata.
There are no external font requests.

## Layout and alignment

Use a narrow flat index, a dominant uninterrupted stage, and a quiet feedback rail:

```text
Conversation index | Visual title / revision   Ctrl K ? | Feedback
                   |                                    | revision context
Visual index       |            rendering               |
                   |                                    | comment
Connection state   | Render state                       | Add / Ctrl Enter
```

Navigation is left aligned; rendering composition belongs to its author. A
selection is an indexed row with an edge marker, not a rounded card. Pane borders
mark real navigation boundaries. Small controls and larger dialogs use distinct
radii; the visual stage has no enclosing card or decorative grid. At narrower
widths feedback moves below the stage and keyboard pane shortcuts still reach it.

## Review against the brief

The previous dotted drafting grid, rounded selected cards, and oversized heading
read as a generic viewer shell. Remove those devices. Keep the existing hatched
frame mark as the single recognizable brand detail, reduce the surrounding chrome,
and spend visual emphasis on the displayed idea and visible keyboard focus.
Reject a hero section, stats, gradients, and decorative motion: none helps the
person inspect a visual or give revision-linked feedback.

Live ultrawide inspection showed natural-size media stranded at the top-left.
Center diagrams and images in the projection stage without distorting them or
changing their authored colors. Keep HTML composition under its author's control.

## Interaction and content

The searchable switcher, pane shortcuts, revision keys, comment submission, and
in-app shortcut discovery are primary. Publishing preserves focus. App navigation
shortcuts remain available inside isolated renderings, and historical selection
and drafts retain their revision identity. Use actual keyboard hints where they
help an action, without adding redundant labels everywhere.

Dark is the default for the window, stage, forms, dialogs, and Mermaid renderer.
Do not invert images or override explicitly authored visual colors. Error and
empty-state copy should name the state and give the next useful action.

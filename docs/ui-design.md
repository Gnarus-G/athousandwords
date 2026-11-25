# The visual surface

## Base idea

The app is a canvas. The agent's rendering fills the window; nothing else is
permanently on screen except one status line. Everything else is summoned and
dismissed with the keyboard.

Any addition to that base must appear in the table below with the reason it
cannot be left out. Remove an entry when its reason no longer holds.

## Additions and why they exist

| Addition | Shown | Why the canvas alone is not enough |
| --- | --- | --- |
| Status line (one 28px row) | Always | The person must know which conversation and visual they are looking at, because comments attach to that exact revision ([ADR 0001](adr/0001-revision-linked-feedback.md)). |
| Revision picker and "of N" | With a visual | Viewing an older revision must be visible; otherwise a comment could silently target the wrong version. |
| "Show latest (revision N)" | Only when behind | Returns a pinned historical view to live updates in one click. |
| Description | When the agent sent one | Agent-supplied context; truncated, full text on hover. |
| Render status | Only while rendering or after failure | A failed rendering must not look like an empty canvas. Success shows nothing. |
| Error | Only on error | Disconnection or an API failure must not be silent. |
| "New visuals in …" | Only when another conversation has unseen visuals | Publications elsewhere never switch the view, so without this they would go unnoticed. One click switches. |
| Feedback button and panel | Button with a visual; panel on demand (`c`) | Comments are the return channel to the agent. The panel floats over the canvas corner instead of taking a column. |
| Quick switcher (Ctrl K) | On demand | Replaces both former sidebars: conversations, visuals, and rare actions (delete) in one searchable list. |
| Shortcut help (`?`) | On demand | Discoverability for a keyboard-first app without labelling every control. |

Removed in the canvas overhaul, with the reason: the conversation and visual
sidebars (moved into the switcher), the permanent feedback column (now a
panel), the brand header, the always-on connection and render status (shown only
when not fine), delete buttons (rare and destructive; switcher only), and pane
cycling with F6 / Alt 1–4 (only the canvas and comment field remain to focus).

## Look

Dark by default, matching the frame and Mermaid theme. Tokens in
`web/style.css`: stage `#181923`, raised overlays `#20212c`, controls
`#2a2c39`, text `#e7e5e0`, quiet text `#aaa8b7`, focus `#b8b2dc`, lines
`#3c3e50`, errors `#f3b2b5`. Cantarell with Noto Sans and Liberation Sans
fallbacks; no external font requests. No gradients, decoration, or motion.

Diagrams and images are centered without distortion or color changes. HTML
composition stays under its author's control.

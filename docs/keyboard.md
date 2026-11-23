# Keyboard-first interaction

The core workflow works without a mouse. The app keeps keyboard focus stable
when an agent publishes, and searches/actions are available from one switcher.

| Keys | Action |
| --- | --- |
| Ctrl K / Cmd K / `/` | Search conversations, visuals in the current conversation, and actions |
| Up / Down, then Enter | Select a switcher result |
| F6 / Shift F6 | Next / previous pane |
| Alt 1 / 2 / 3 / 4 | Conversations / visuals / canvas / comment |
| j / k or Down / Up | Move through a focused navigation list |
| Home / End | First / last item in that list |
| Enter | Select an item; from the canvas, enter the interactive rendering |
| `[` / `]` | Older / newer revision |
| `g` | Follow the latest revision |
| `c` | Focus the comment field |
| Ctrl Enter / Cmd Enter | Add the comment |
| Escape | Dismiss an overlay, or return from an input/rendering to the canvas |
| `?` | Open shortcut help |
| Ctrl W / Ctrl Q | Close the desktop app |

Letter shortcuts leave text fields alone. The desktop app reserves Ctrl/Cmd K,
Escape, F6, and the pane shortcuts even while a sandboxed rendering has focus.
Other keys remain available to the rendering's own interactions.

The switcher and shortcut help are also visible buttons. Switcher results announce
their active option to assistive technology, dialogs contain focus, and closing
them restores it synchronously so the next keystroke acts on the expected pane.

Historical selection and typed comment drafts retain their revision identity.
If a requested revision is still loading, focusing a new comment waits for that
revision rather than silently attaching it to the previous one. Explicitly
selecting an older visual stays selected across polling; only a genuinely new
publication can change a live-following view.

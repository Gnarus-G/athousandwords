# Keyboard-first interaction

The core workflow works without a mouse. Publishing does not steal focus.

| Keys | Action |
| --- | --- |
| Ctrl K / Cmd K / `/` | Search conversations, visuals in the current conversation, and actions |
| Up / Down, then Enter | Select a switcher result |
| Enter | From the canvas, enter the interactive rendering |
| `[` / `]` | Older / newer revision |
| `g` | Follow the latest revision |
| `c` | Open feedback and focus the comment field |
| Ctrl Enter / Cmd Enter | Add the comment |
| Escape | Close an overlay or the feedback panel, or return from a rendering to the canvas |
| `?` | Open shortcut help |
| Ctrl W / Ctrl Q | Close the desktop app |

Letter shortcuts leave text fields alone. The desktop app reserves Ctrl/Cmd K
and Escape even while a sandboxed rendering has focus. Other keys remain
available to the rendering's own interactions.

Switcher results announce their active option to assistive technology, dialogs
contain focus, and closing them restores it synchronously.

Historical selection and typed comment drafts keep their revision identity.
If a requested revision is still loading, focusing a new comment waits for that
revision rather than silently attaching it to the previous one. Explicitly
selecting an older visual stays selected across polling; only a new publication
can change a live-following view.

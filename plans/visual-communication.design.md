# A Thousand Words: first-version design

Status: implemented and verified within the limits recorded below. User confirmed no license grant for project code.

## Goal

Give agents a visual communication channel comparable in ease to TTS or Markdown
code blocks, independent of which harness they use.

## Architecture

```text
Local harness  -- MCP adapter or CLI ------------------+
                                                      |
Remote harness -- MCP adapter or CLI -- SSH forwarding +--> Local Rust companion
                                                               |
                                                        Persistent workspaces
                                                               |
                                                        Dedicated desktop app
                                                               |
                                                       Revision-linked comments
                                                               |
                                          Agent reads or waits through MCP / CLI
```

The local companion outlives harness sessions. Publishing opens a dedicated desktop
app when none is connected and reuses it for later visuals; closing it and publishing
again reopens it. It does not hand off to the default browser or automatically switch
the workspace the person is viewing. This replaces the original manual/browser-first
decision following direct user feedback.
Initial support covers a Linux desktop and local/remote Linux agents, without a
required hosted service or account.

MCP and CLI expose the same application capabilities. Each agent connection is
scoped to a conversation workspace, which can be resumed across connections. The
person's UI can browse all workspaces and identify their originating conversations.
Remote adapters transfer supplied content, including image bytes; local and remote
machines need no shared filesystem.

## Visuals and presentation

- Initial formats: Mermaid, SVG, PNG/JPEG/WebP, and self-contained HTML/CSS/JavaScript.
- The agent supplies content or supplies a file to its adapter. Content generation,
  desktop capture, and conversion of other formats remain external capabilities.
- Interactive content is isolated from the filesystem, external network, and the
  companion's control interface. Companion comment controls sit outside renderings.
- Creating a visual assigns a stable identity; updates create retained revisions.
- Updates must name the expected current revision. Stale replacements fail with a
  conflict, preserving the newer work.
- Viewing latest follows live updates. Selecting a historical revision fixes the
  view there until the person changes it; indicate when newer content exists.
- Content arriving in another workspace produces an unread indication without
  switching the current workspace.
- With the viewer closed, accept and retain content. On reconnect, present latest
  content, keeping intervening revisions in history rather than replaying them.
- Keep acceptance, viewer presence, render confirmation, and render errors distinct.
  Report a revision as rendered only after viewer acknowledgment. That acknowledgment
  establishes rendering, not that the person looked at the result. Publishing an
  inactive visual must not be mislabeled as rendered merely because a viewer exists.

## Feedback and persistence

The GUI is keyboard-first: core navigation, visual/revision selection, comments,
and shortcut discovery require no mouse. The dedicated app routes its reserved
navigation shortcuts even when a sandboxed rendering owns focus. See
[the keyboard contract](../docs/keyboard.md).

- Initial feedback is a whole-visual comment attached to the exact revision viewed.
- Agents read comments after a continuation position or wait for new feedback with
  a bounded timeout. Reading does not delete comments.
- Resume from that position after disconnection. Interrupted calls may replay
  comments; retrieval must not silently skip them.
- Persist workspaces, visuals, revision history, assets, and comments locally across
  companion restart until explicitly deleted.
- Provide explicit deletion through the application interfaces. Deletion semantics
  and reconnect behavior must be documented and tested against the retained data.

## First-version capabilities

1. Create and resume a conversation workspace, and connect an adapter to it.
2. Show a visual; list and inspect the workspace's visuals and revisions.
3. Replace a visual using an expected revision, and retrieve presentation status.
4. Browse workspaces/history and add revision-linked comments in the desktop app.
5. Read or wait for feedback through either MCP or CLI.
6. Delete retained workspace content explicitly.
7. Supply setup instructions, SSH-forwarding guidance, examples, and a portable agent
   instruction encouraging visuals when they communicate an idea better than prose.

Command names, storage technology, wire encoding, local pairing mechanism, timeouts,
and bounded resource limits are engineering details, not decisions settled by the
interview. They must preserve the behavior above and be documented with the implementation.

## Acceptance checks

The checks below were exercised as recorded in [the verification record](visual-communication.verification.md), which states the tested topology and limits:

- [x] A real MCP client discovers and calls tools; a browser renders each initial
      content format. The CLI exercises the same application capabilities.
- [x] An adapter on a separate Linux machine, or an explicitly identified SSH test
      environment, transfers content through SSH forwarding to the local companion.
      State clearly which topology was actually exercised.
- [x] A visual updates in place with retained revisions. A stale replacement fails
      without altering newer content.
- [x] Latest follows updates; deliberately viewed history stays fixed when a newer
      revision arrives.
- [x] Two conversation workspaces remain separate; an agent cannot operate in the
      other's workspace through its scoped connection. New content does not switch
      the person's active workspace.
- [x] A comment targets the viewed revision even if an update arrives concurrently.
- [x] Feedback read, bounded wait, timeout, and resume work across disconnects,
      without destructive reads or skipped comments in the exercised cases.
- [x] Companion restart preserves workspaces, contents, revisions, and comments;
      explicit deletion removes the intended retained content.
- [x] Closing the viewer does not lose published content; publishing reopens the
      dedicated app, which displays latest content with earlier revisions retained.
- [x] Malformed renderable content produces a useful rendering failure. Acceptance
      and render acknowledgment remain distinct, including inactive workspace cases.
- [x] Interactive HTML works within the agreed boundary; representative attempts to
      use external network, filesystem, or companion controls are refused. Report
      tested mechanisms and limits rather than claiming exhaustive proof.
- [x] Setup and usage examples demonstrate the visual-and-feedback loop, including
      remote assets, workspace resumption, and proactive agent usage guidance.

## Deferred

Inline MCP Apps delivery; non-Linux platform support;
point/region feedback and freehand annotation; screenshots of rendered visuals
returned to the agent; automatic idle-harness wake-up; non-SSH remote connectivity;
native handling of additional media formats.

## Design records

- [Goal](visual-communication.goal.md)
- [Interview decision tree](visual-communication.design-tree.md)
- [Research](visual-communication.research.md)
- [Glossary](../GLOSSARY.md)
- [Revision-linked feedback ADR](../docs/adr/0001-revision-linked-feedback.md)
- [Local companion ADR](../docs/adr/0002-companion-owns-presentation.md)

# Visual communication design interview

Status: implemented; subsequent user corrections replace browser-first/manual launch with a dedicated automatically opened desktop app. See ADR 0004 and the goal file.

## Settled from the user's request

- Goal: visual communication available to LLMs across harnesses, comparable in ease to TTS or Markdown code blocks.
- Useful output includes diagrams, screenshots, drawings, and other renderings.
- Delivery technology is open to investigation.

## Round 1: settled

The user answered “yes” to the four recommendations:

1. Proactively display visuals on a reusable surface, without stealing keyboard focus.
2. Include remote agents displaying on the user's local machine in the first usable version.
3. Include simple visual-linked human feedback initially; freehand annotation can follow.
4. Present agent-supplied content; generation and screenshot capture remain external capabilities.

## Round 2: settled

The user answered “yes” to all six recommendations:

1. Local companion with an SSH-forwarded connection for remote agents; no required hosted relay or account.
2. Browser surface first; native window may follow.
3. One companion with separate conversation workspaces and visible provenance.
4. Update visuals in place while preserving earlier versions.
5. Explicit read/wait tools for feedback; automatic idle-harness wake-up is not required initially.
6. Self-contained interactive JavaScript in isolated renderings, without network, filesystem, or companion-control access. Companion feedback controls sit outside the rendering.

## Round 3: settled

The user answered “yes” to all six recommendations:

1. Companion outlives agent sessions; visuals, revisions, and feedback persist locally until explicit deletion.
2. Other conversations publish without switching the workspace being viewed; indicate new content instead.
3. Initial feedback is whole-visual comments tied to the exact revision viewed; spatial annotations follow later.
4. Remote agents transfer explicitly supplied asset contents; no shared filesystem or companion access to remote paths required.
5. Distinguish accepted, rendered, rendering failed, and no viewer. Rendered requires viewer confirmation and does not imply human attention.
6. Initial supported platform is Linux for the desktop and remote agents; retain portable interfaces.

## Round 4: settled

The user answered “yes” to all six recommendations:

1. Build our own focused Rust companion, using existing rendering libraries.
2. Provide MCP and CLI interfaces with the same capabilities and workspace model.
3. One deliberate companion/viewer launch; tool calls never open or focus a browser window. Report no viewer if it is closed.
4. Agent connections are scoped to their conversation's workspace; the user UI can browse all workspaces. Resuming a conversation reuses its workspace.
5. Non-destructive, durable cursor-based feedback retrieval with bounded wait. Interrupted calls may replay comments rather than lose them.
6. Initial formats: Mermaid, SVG, PNG/JPEG/WebP, and self-contained HTML/CSS/JavaScript. Other formats use external conversion initially.

## Round 5: settled

The user answered “yes” to all four recommendations:

1. Follow updates while viewing latest; keep a deliberately selected historical revision fixed, with a newer-version indicator.
2. Replacements specify the expected prior revision; reject stale updates with a conflict rather than silently overwrite newer work.
3. Accept and retain content without a viewer; show latest content on reconnect, retaining intervening revisions without slideshow replay.
4. Initial agent inspection uses render acknowledgment and errors. Screenshots of rendered visuals returned to the agent are a later capability.

## Branch disposition

- Presentation: dedicated desktop app opens on publication when absent; startup requests coalesce and a connected instance is reused. Retained history and latest/history navigation remain; publishing does not switch an active workspace.
- Topology: Linux local companion plus remote Linux adapters over SSH forwarding; conversation-scoped access and explicit content transfer.
- Feedback: whole-visual revision-linked comments, retained locally and retrieved through resumable read or bounded wait.
- Content: initial formats and isolation settled; generation/capture external; agent-facing render screenshots deferred.
- Ownership: custom Rust implementation, shared MCP/CLI capabilities; existing products remain references.
- Open: final confirmation of [the consolidated design and acceptance checks](visual-communication.design.md).
- Engineering details such as storage library, wire encoding, local pairing mechanism, and resource limits will be chosen to satisfy this contract; no particular choices have been approved in this interview.

## Documentation policy

- Record settled terms in root GLOSSARY.md when they crystallize.
- Record an ADR only for a consequential, hard-to-reverse trade-off needing rationale.
- Browser-first delivery and Rust are approved. Implementation still awaits final shared-understanding confirmation.

# A Thousand Words: research and proposed first slice

Research date: 2026-11-23. This document records the initial proposal; subsequent implementation and verification are recorded separately.

Subsequent interview decisions are authoritative in [the design tree](visual-communication.design-tree.md)
and [goal file](visual-communication.goal.md). The initial recommendations below are retained as research context.

## Product contract

An agent should be able to say “show this” through a tool, just as it can speak
through TTS. The result appears on a reusable visual surface. Further tool calls
can revise that visual in place. Saving a file or returning an image to the model
alone does not establish that the user saw it.

MCP provides a common tool interface, but does not guarantee that every harness
displays image results or interactive apps the same way. Shell-capable harnesses
without MCP can use a CLI. Hosted agents need an explicit network path to the
viewer; their localhost is not the user's desktop.

## Existing work

These are documentation findings, not hands-on evaluations:

| Option | Documented capabilities | Fit / remaining question |
| --- | --- | --- |
| [QuickShow](https://github.com/zolat/quickshow) | Floating panels; Markdown, SVG, Mermaid, images, HTML; screenshots back to the agent; annotation feedback | Very close product reference. Distributed app is Apple-Silicon-only; not a direct Linux installation. |
| [Sideshow](https://sideshow.sh/) | MCP-connected browser boards; HTML, diagrams, images, comments; local and hosted modes | Worth evaluating before duplicating it. Does its board workflow provide the immediate show/update interaction we want? Site says screenshots are unavailable in local Node mode. |
| [whiteboard-mcp](https://github.com/mikewei/whiteboard-mcp) | Browser whiteboard; HTML, Markdown, URLs; history; HTTP MCP and REST | Small baseline worth evaluating; documented tools are narrower than the intended multi-format visual channel. |
| [MCP Apps](https://apps.extensions.modelcontextprotocol.io/api/documents/overview.html) | Standard tool-linked HTML resources rendered in a host iframe; bidirectional interaction | Useful additional adapter. Host support varies, so it cannot by itself satisfy arbitrary-harness delivery. |

## Recommended architecture if building our own

```text
MCP-capable harness -- MCP adapter --+
                                    +-- local visual service -- live browser viewer
Shell-capable harness ---- CLI -----+
```

Use Rust for the service and CLI, the official [rmcp SDK](https://github.com/modelcontextprotocol/rust-sdk)
for MCP, and a small web frontend for rendering. Keep rendering and document state
independent of the adapter. A single viewer service lets separate harness
processes share a surface without sharing or overwriting each other's documents.

Start with the installed browser: it already renders HTML, SVG, images, and
canvas content, and can stay beside the conversation. Bundle rendering dependencies
locally rather than requiring a CDN. A desktop shell can reuse the frontend later.
[Tauri's Linux prerequisites](https://v2.tauri.app/start/prerequisites/)
include WebKitGTK; it adds platform packaging work before proving the communication
loop, although it could later supply a dedicated window and desktop integration.

### Small public interface

- `show_visual`: title, format, content or local file, optional existing visual ID,
  and a short text description. Return visual ID and viewer URL. Reusing an ID
  replaces that visual; a new ID creates another.
- `list_visuals`: discover the calling session's visuals for later updates.
- Equivalent CLI operations for harnesses without MCP.

The frontend should keep a history/sidebar, identify the originating session, and
update an open visual live. Open the viewer when needed; avoid spawning a new tab
on every update. Opening a URL is not proof that it rendered: track browser render
acknowledgments and rendering errors separately from server acceptance.

### First formats

- Existing image files for screenshots and generated plots. Screenshot capture
  remains a separate tool; this program displays its result.
- SVG for drawings and precise diagrams.
- Mermaid for quick structure and sequence diagrams.
- Sandboxed HTML/CSS for layouts and other custom renderings. Decide whether
  scripts are needed in the first slice; do not give generated content access to
  the service's control API or the filesystem.

### First verification

1. A real MCP client discovers and calls `show_visual`; the browser renders a
   diagram and reports completion.
2. A second call updates that visual in the existing viewer.
3. An existing screenshot and an HTML rendering also display correctly.
4. Invalid content produces a useful error rather than a false rendered status.
5. Separate sessions cannot accidentally replace one another's visuals.

### Follow-up capabilities

- Return a screenshot of the actual rendered result to the agent so it can inspect
  readability and layout. This needs a capture implementation; MCP alone does not
  provide it.
- Human annotation and structured feedback. Expose feedback through an explicit
  read/wait tool first; do not assume every harness can accept unsolicited turns.
- MCP Apps delivery in supporting hosts, using the same document model.
- Native window/tray behavior, export, and remote-agent/local-viewer pairing.

## Adoption

Tools enable the capability but do not ensure agents choose it. Supply a portable
agent instruction: use a visual when it explains relationships, spatial structure,
comparisons, or a sequence more clearly than prose; include a short explanation;
update the existing visual when refining the same idea. Do not visualize everything.

## Decision needed

Confirm browser-first Rust implementation and whether the first required topology
is agents running on this desktop or remote agents communicating with this desktop.
Evaluate Sideshow first if reuse is preferred to owning a small custom application.

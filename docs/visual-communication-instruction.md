# Visual communication

Use the visual companion proactively when a diagram, drawing, image, or rendering
explains an idea more clearly than prose: relationships, sequences, spatial
structure, comparisons, and concrete UI examples are good candidates.

- Use `show_visual` through MCP, or `atw --connection ... show` through the shell.
  Use the connection assigned to this conversation; do not read or print its contents.
- Give each visual a meaningful title and a short explanation. Prefer Mermaid for
  structure/sequences, SVG for precise drawings, image files for screenshots/plots,
  and self-contained HTML for interactive explanations. No CDN or network access.
- Use `local_file` for files on your machine, including remote machines. The adapter
  transfers the content to the viewer. Image generation and capture use other tools.
- Refine an existing visual with its ID and expected revision. Inspect a conflict
  before replacing again; do not blindly overwrite the current version.
- Acceptance is not rendering. Query `visual_status`; report rendering errors
  accurately. Publishing automatically opens the dedicated desktop app if needed;
  check `presentation.opening` and allow it to connect before asking the person to
  do anything. If launching fails, report the error and offer `atw open`. Reuse the
  app for subsequent visuals rather than opening a webpage in a browser.
- Read visual feedback and keep the returned cursor for subsequent reads/waits.
  A comment refers to its exact revision, which may no longer be latest. Reads are
  non-destructive; deduplicate replayed comment IDs after interruptions.
- Give keyboard directions for human interaction: Ctrl K switches, ? shows help,
  c focuses a comment, and Ctrl Enter sends it.
- Use visuals where they add clarity, not for every response. Keep brief prose in
  the conversation so the person understands why you are showing something.

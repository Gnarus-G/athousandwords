# First-version verification

Observed on 2026-11-23 in this Linux workspace. This records exercised behavior,
not a claim of exhaustive correctness or compatibility with every harness.

## Final checks

| Check | Observed result |
| --- | --- |
| `npm run build` | Bundled the local Mermaid/renderer asset successfully |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --locked` | 12 integration tests passed |
| `cargo build --locked` | Built `target/debug/atw` with embedded viewer assets |
| `npm test` | 10 Chromium browser tests passed |
| `npm run test:desktop` | Dedicated Electron window, isolation, keyboard routing, and clean close passed |
| `npm run test:ssh` | Passed actual SSH execution and reverse-forwarded SVG transfer |

Environment observed: Rust 1.93.1, Node 26.13.1, Playwright Chromium 159 for
headless regression tests, and Electron 45.4.1 with its own Chromium 156 for the
dedicated desktop app. The app does not depend on the default browser's engine
or extensions. Connection-Allowlist support is documented by
[the browser vendor](https://developer.chrome.com/blog/connection-allowlist-announcement).

## Exercised flows

- A JSON-RPC MCP client initialized the real stdio server, discovered tools,
  published a visual, and obtained an object-shaped structured list result.
- An additional full round trip used a real stdio MCP tool call with a local
  Mermaid file, observed it render in the browser, entered a comment in the UI,
  and retrieved that revision-linked comment through the MCP feedback tool.
- A real CLI subprocess read a local SVG file, transferred its contents, and
  returned visual identity/revision without printing its workspace credential.
- HTTP integration exercised workspace isolation, retained revisions, stale-update
  conflicts, viewer presence separate from render acknowledgment, and explicit
  deletion of visuals/comments/workspaces. Workspace deletion remained effective
  after reopening the persistent companion.
- Revision-linked comments survived a companion restart. Reading was
  non-destructive; bounded waiting, timeout, interrupted-request retry, and
  resuming from a prior cursor after restart were exercised.
- Browser flows rendered Mermaid, SVG, PNG, JPEG, WebP, and interactive HTML.
  Malformed Mermaid and SVG produced failed rendering status with useful errors.
- Latest followed live updates; historical revision selection stayed fixed. A
  comment draft retained the revision at which typing began despite a live update.
- Another conversation remained unread without switching the active workspace;
  its accepted publication was not mislabeled as rendered.
- Closing the viewer expired its presence; publication still succeeded. Reopening
  showed latest content, with intermediate revisions retained.
- A delayed fetch reproduced the wrong-visual deletion race before the fix. The
  regression test now verifies deletion targets the displayed visual.
- A deliberately dropped render-acknowledgment POST was retried successfully,
  without replacing the visual to force another render.
- A sandboxed HTML script could use its own button but could not read parent
  owner state, fetch an innocuous local file, or contact the control API/external
  HTTP probe. External fetch/image/navigation attempts caused zero probe requests.
- A WebRTC STUN probe initially received two UDP packets with CSP alone. With the
  Connection-Allowlist boundary, the same test received zero packets.
- A real Electron window rendered the diagram and interactive HTML, had no Node
  APIs in either the main page or generated content, and blocked the UDP probe.
- Publishing requests a desktop launch when absent, coalesces requests while
  starting, reuses a connected app, and requests reopening after closure. Launch
  failures preserve stored content. A legacy browser tab cannot suppress the
  app or overwrite desktop render evidence.
- The keyboard-only workflow searches conversations and visuals, navigates results
  and panes, selects retained revisions, and submits revision-linked comments.
  Explicit visual selection survives polling and focus survives live updates.
- Fast revision commands use selected visual metadata rather than stale controls;
  comments wait for the intended revision. The focused test passed three repeated
  runs before the full ten-test browser suite passed.
- Ctrl K and F6 were exercised through Electron's native input route with focus in
  an interactive rendering; Escape returns to the canvas and Ctrl W closes cleanly.
  Chromium CDP key injection bypasses Electron's before-input-event hook, so the
  desktop test uses webContents.sendInputEvent for the reserved native shortcuts.
- Delaying viewer deregistration by 250 milliseconds reproduced stale presence
  after closing. The app now waits for normal deregistration before destruction;
  the desktop test observes disconnected state immediately after keyboard close.

## SSH topology

The test created a disposable loopback sshd and temporary keys, opened a real
reverse SSH forward, executed the CLI through SSH, and verified transferred SVG
bytes through the companion API. Companion data and the adapter's image were in
separate temporary directories. The test changed no permanent SSH configuration.
It disabled StrictModes only for this disposable daemon because its authorized
test key lives beneath `/tmp`.

This was not a separate physical machine or a network-namespace isolation test.
Real remote host ABI, SSH account policy, and forwarding permissions still depend
on that deployment's setup.

## Review and artifacts

Independent standards and spec reviews identified the empty-credential startup
case, output-file reservation ordering, deleted-workspace errors, presentation
races, lost acknowledgments, duplicate-title provenance, MCP list shape, and the
CSP/WebRTC gap. These led to focused fixes and the checks above.

The browser suite generates `test-results/visual-companion.png`; the desktop suite
generates `test-results/desktop-companion.png`. These git-ignored artifacts are
regenerated by their corresponding suites. The installed app was also observed
as a compositor window with class `athousandwords` and title `A Thousand Words`,
after a real MCP publication automatically opened it and render status became
`rendered` with a connected desktop viewer.

After the keyboard upgrade was installed, a real MCP call revised “How A Thousand
Words works” to revision 3. Its response reported a requested app launch; a later
status query reported `rendered` and `viewer_connected: true`. A compositor-window
capture confirmed the dedicated app displayed that revision with Switch/Ctrl K
and Keys/? controls, without browser chrome. The capture is an agent-owned temporary
artifact at `/tmp/opencode/athousandwords-desktop.png`.

## Limits and licensing

- MCP was exercised through a real protocol client, not by installing configuration
  into every named harness. A harness that shares one server entry across
  conversations also shares its configured workspace; connection setup must reflect
  the desired conversation boundary.
- Browser checks establish the tested mechanisms/cases, not protection against
  browser vulnerabilities, extensions, or unbounded generated JavaScript execution.
- No macOS/Windows, inline MCP Apps, spatial annotations, rendered
  screenshots returned to agents, or automatic idle-harness wake-up was implemented.
- Playwright's Electron driver adds its automation/debugging flags (including
  --no-sandbox); the production launcher does not add that flag. The installed
  application was separately launched through the running companion and observed.
- The user clarified that “unlicensed” means no license grant. There is no project
  LICENSE file; npm metadata is UNLICENSED and Cargo has no license declaration.
  Third-party dependencies retain their own notices and terms.

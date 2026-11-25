# A Thousand Words

A visual companion for agents: show an idea, revise it, and discuss the exact
version you are looking at. A local Rust service keeps the visuals; a dedicated
Electron desktop app is the reusable surface. Agents use MCP or the `atw` CLI,
locally or through SSH. Publishing opens the app automatically when needed.

## Build and start

Initial platform: Linux. Build prerequisites: Rust 1.93+, Node.js/npm, and a C
toolchain for bundled SQLite. The desktop app includes Electron 45.4.1 and its
Chromium 156 engine, independently of your default browser and its extensions.
Dependencies are downloaded at build time; rendering needs no CDN or hosted account.

```sh
npm ci
node node_modules/electron/install.js
npm run build
cargo build --locked

# Leave this running in its own terminal.
./target/debug/atw serve
```

In another terminal, create a conversation and publish a visual. The app opens:

```sh
./target/debug/atw workspace create --title "Architecture discussion" --out ./architecture.atw.json
./target/debug/atw --connection ./architecture.atw.json show \
  --title "The communication loop" --format mermaid --file examples/loop.mmd
```

The connection file contains a workspace credential. Keep it out of source control
and pass its **path**, not its contents, to a harness. Creation uses mode `0600`
and refuses to overwrite an existing file. Reuse that file to resume a conversation.
The companion defaults to `http://127.0.0.1:7331`; `--url` overrides the adapter
endpoint, and `serve --port` selects a listening port.

Publishing opens the dedicated desktop app if none is connected; subsequent calls
reuse the window. Closing it and publishing again reopens it. Startup requests are
coalesced for ten seconds to avoid duplicate launches. The app also enforces one
instance per companion data directory. `atw open` opens or raises it explicitly.
Use `atw serve --no-open` for headless operation. A legacy browser tab does not
suppress opening the desktop app.

The app's trusted main process reads the local owner credential. Credentials are
not command-line arguments or exposed through a Node/preload bridge. The embedded
UI consumes the credential from a local URL fragment into its own session storage.
The desktop runtime currently runs from this checkout's `node_modules`; keep it
alongside the built Rust application.

## MCP

Add a stdio server using your harness's MCP configuration syntax. A common shape is:

```json
{
  "mcpServers": {
    "visuals": {
      "command": "/absolute/path/to/athousandwords/target/debug/atw",
      "args": ["--connection", "/absolute/path/to/architecture.atw.json", "mcp"]
    }
  }
}
```

Use one workspace connection per conversation. MCP itself does not provide a
universal conversation identity: if a harness reuses one configured server across
conversations, that entry deliberately shares its workspace. Launch separate
entries/processes with separate connection files when isolation is needed.
Owner workspace creation/listing is setup through the local CLI; scoped agents
cannot mint credentials for other workspaces.

Tools:

| Tool | Purpose |
| --- | --- |
| `show_visual` | Create a visual, or replace one with `id` and `expected_revision` |
| `list_visuals` | Discover visuals and their current revisions |
| `list_revisions` | Browse retained versions |
| `get_visual` | Read an exact revision's content and description |
| `visual_status` | Inspect viewer presence, render acknowledgment, and rendering errors |
| `read_feedback` | Non-destructively read after a cursor, optionally waiting up to 30 seconds |
| `add_comment` | Attach a comment to an exact revision |
| `delete_visual` | Delete a visual, all revisions, and associated comments |
| `delete_workspace` | Delete this workspace and revoke its connection |

For example, an agent can call:

```json
{
  "title": "A request through the system",
  "format": "mermaid",
  "content": "sequenceDiagram\n  Person->>Agent: Explain the flow\n  Agent->>Companion: Show a diagram\n  Companion-->>Person: Render it"
}
```

Supply exactly one of `content` or `local_file`. For image files, prefer
`local_file`: the adapter reads and transfers bytes on the agent's machine,
without spending model tokens on base64. Inline PNG/JPEG/WebP `content` is base64;
Mermaid, SVG, and HTML use UTF-8 source. List results retain JSON arrays in text
content and use `{ "items": [...] }` in structured MCP content.

## CLI

```sh
atw --connection conversation.atw.json list
atw --connection conversation.atw.json history VISUAL_ID
atw --connection conversation.atw.json get VISUAL_ID 1
atw --connection conversation.atw.json status VISUAL_ID 1
atw --connection conversation.atw.json show --title "Refined flow" \
  --format mermaid --file refined.mmd --id VISUAL_ID --expected-revision 1
atw --connection conversation.atw.json feedback --after 0 --wait-ms 30000
atw --connection conversation.atw.json comment VISUAL_ID 1 "Explain this arrow."
atw --connection conversation.atw.json delete VISUAL_ID
atw --connection conversation.atw.json delete-workspace
```

Here `atw` means the built binary, or a copy on your PATH. Omit `--file` to read
source or image bytes from stdin. `ATW_CONNECTION`, `ATW_URL`, and `ATW_DATA_DIR`
are alternatives to the global flags. Paths containing spaces should be quoted.

Save the returned feedback `cursor` and pass it as `--after` on the next read or
wait. Pages contain at most 100 comments. An interrupted read can be retried with
the previous cursor; comments can replay and must be deduplicated by ID, but a
successful read does not remove them. A timed-out wait returns an empty page and
the same cursor. Deleting a visual removes its comments; deleting a workspace
invalidates its credential, including reconnect attempts.

## A remote agent, a local screen

Run the companion on the desktop. Install/copy the `atw` binary onto the remote
Linux machine (build there when its ABI differs), and transfer **only the
conversation's connection file**, not the companion's owner credential.

From the desktop, establish a reverse forward:

```sh
ssh -N -o ExitOnForwardFailure=yes -R 127.0.0.1:17331:127.0.0.1:7331 user@remote
```

On the remote machine, the adapter uses that forwarded endpoint:

```sh
atw --connection ./conversation.atw.json --url http://127.0.0.1:17331 \
  show --title "A remote drawing" --format svg --file ./drawing.svg

atw --connection ./conversation.atw.json --url http://127.0.0.1:17331 mcp
```

The same extra `--url` arguments work in the remote harness's MCP entry. The SSH
connection must remain open; the server must permit forwarding. The image/source
file is read remotely and sent in the request, not opened by the desktop service.
No shared directory, public listener, or cloud relay is needed.

## Presentation and feedback

The GUI is a full-window canvas with one status line. **Ctrl K** opens the
searchable switcher for conversations, visuals, and actions; **?** shows all
shortcuts, **[ / ]** changes revisions, **c** opens feedback, and **Ctrl Enter**
adds a comment. Ctrl K and Escape also work when focus is inside a rendering.
See [the keyboard guide](docs/keyboard.md) and [the UI design](docs/ui-design.md).

- Separate conversations have their own visual lists. Other conversations show
  unread indicators without switching the current view. Short workspace IDs
  distinguish conversations with identical titles.
- The latest view follows revisions. Choosing a historical revision pins it;
  **Show latest** returns to live updates. Stale replacements return a conflict
  rather than overwrite another operation's work.
- A comment draft captures its visual and revision when typing starts, even if
  another update arrives before submission. Its target remains visible beside
  the comment controls.
- `presentation.viewer_connected` is a five-second, heartbeat-based presence
  lease. `render_status` is `accepted`, `rendered`, or `failed`; it records the
  last acknowledgment for that revision, separately from current presence.
  Acceptance and a connected viewer alone do not establish rendering. Rendering
  does not establish that the person looked at it. A historical acknowledgment
  remains historical after closing or restarting the viewer.
- With the viewer closed, publications remain stored. Reconnecting shows latest
  content, retaining intermediate revisions in history instead of replaying them.
- A publish result also reports `presentation.opening.status`: `requested`,
  `pending`, `already_connected`, `failed`, or `disabled`. Requesting a launch does
  not establish that the app connected or rendered; use the render status for that.

## Rendering boundary and limits

Renderings run in an opaque-origin iframe with scripts enabled and no other
sandbox grants. Companion navigation and comments remain outside that frame.
CSP restricts resources, nesting, forms and control connections. The frame's
`Connection-Allowlist` permits only the bundled renderer asset; WebRTC and other
network destinations are denied by the bundled engine. This addresses a gap
in CSP alone; see [Chrome's documentation](https://developer.chrome.com/blog/connection-allowlist-announcement).
The desktop window has Node integration disabled, context isolation and sandboxing
enabled, no preload bridge, denied permissions/popups/downloads, and only permits
network traffic to its own local companion. Arbitrary engine exploits and denial
of service from unbounded JavaScript execution are not established by our tests.

No external scripts, stylesheets, fonts, URLs, or CDN imports are needed/supported
inside renderings. Inline scripts and embedded data images work. Screenshot
capture, generation models, and file conversion remain separate agent tools.

Local file/stdin input is limited to 8 MiB. The HTTP JSON body limit is 13 MiB,
with a 12 MiB content-field limit; JSON escaping can reduce the effective source
limit. Titles are limited to 200 UTF-8 bytes, comments to 16 KiB, and feedback waits
to 30 seconds. Stored history has no automatic expiry or total quota in this
single-user version; delete it explicitly when no longer needed.

## Storage

Default: `$XDG_DATA_HOME/athousandwords`, or
`~/.local/share/athousandwords`. The directory contains `admin.token` and
`companion.sqlite3` (plus SQLite journal files). The default newly created data
directory is owner-only. Preserve the directory and workspace connection files
to resume after restart. A missing/invalid owner credential is not treated as an
anonymous connection. Do not run two companion processes against the same data
directory; use a separate directory and port for a separate companion.
When changing the listening port for an existing data directory, close its old
desktop window before reopening against the new endpoint.

Deletion removes application records and their dependent content via SQLite
foreign-key cascades. It is logical deletion, not a disk-forensics secure erase.

## Verification

```sh
npm run build
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --locked
npx playwright install chromium
npm test
npm run test:desktop

# Optional integration environment: OpenSSH client, ssh-keygen, and sshd.
npm run test:ssh
```

The SSH check creates a temporary loopback sshd with disposable keys and executes
the adapter through a real reverse forward. It is not a separate physical-machine
test. Tests use temporary application data and do not require personal credentials.
See [the acceptance record](plans/visual-communication.verification.md) for observed
results and remaining limits.

## Encourage agents to use it

Add [the portable usage instruction](docs/visual-communication-instruction.md) to
your harness instructions after configuring the connection. The program supplies
the capability; the instruction helps an agent choose it without being asked.

Design: [glossary](GLOSSARY.md), [contract](plans/visual-communication.design.md),
and [decisions](docs/adr/).

## Licensing

This project's own code has **no license grant**. There is no project LICENSE file;
npm metadata is `UNLICENSED` and Cargo has no license declaration. Third-party
dependencies retain their own licenses and notices.

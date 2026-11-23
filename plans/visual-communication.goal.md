# Visual communication

Goal: Give LLMs in different harnesses a visual communication channel as easy to use as TTS or Markdown code blocks.
Done when: MCP and CLI deliver and revise visuals locally and through SSH; publication opens a dedicated keyboard-first app, confirms rendering, preserves history across restart, and returns revision-linked comments through resumable read/wait operations.

## Facts
- The user wants diagrams, screenshots, drawings, and other renderings, choosing whatever communicates an idea best.
- A dedicated desktop app and a custom Rust companion are required. The user corrected the browser-first approach after trying it.
- The user accepted all four round-one recommendations: proactive display without focus theft; remote agents in the first usable version; simple visual-linked feedback; presentation of externally supplied content.
- Reuse the visual surface rather than requiring manual link navigation for each visual.
- Content generation and screenshot capture are separate capabilities, outside this program's initial responsibility.
- The user accepted SSH-forwarded remote connections without required hosting/accounts; one companion with separate conversation workspaces; stable visuals with revision history; explicit feedback read/wait; isolated self-contained interactive JavaScript.
- The user accepted round three: local persistence until explicit deletion; unread indications rather than workspace switching; whole-visual revision-linked comments; explicit remote asset transfer; distinct acceptance/render/error/no-viewer states; Linux desktop and remote agents first.
- Current launch behavior: publishing opens the dedicated desktop app when none is connected, including on first use and after closing it; reuse an existing app window without repeated launches.
- The user accepted round five: follow latest without disrupting history viewing; reject stale replacements; store content with no viewer and show latest on reconnect; defer agent-facing render screenshots.
- Implementation finding: CSP alone allowed WebRTC traffic. The dedicated app supplies Chromium 156 with Connection-Allowlist, independently of the default browser.
- User requested OpenCode setup: installed globally through the dotfiles home profile, with a home-only user service and a shared “OpenCode on home” workspace. Details: opencode-setup.md.
- Keyboard-first GUI is a priority: core navigation, visual/revision selection, comments, and shortcut discovery must be usable without a mouse.

## Agreed first-version scope (approved for implementation)
- A local visual viewer, MCP adapter, and CLI calling the same application logic.
- Show and update named visuals; images, SVG, Mermaid, and sandboxed HTML.
- Remote-agent delivery and simple visual-linked human feedback.
- Test the MCP-to-viewer path and updating an existing visual.
- Prove the full contract using acceptance checks in visual-communication.design.md.
- Document the agent instruction that encourages proactive visual communication.

## Later candidates and remaining design
- Inline MCP Apps delivery and freehand visual annotations.
- Agent-facing screenshots of rendered visuals and automatic idle-harness wake-up.
- Implementation and acceptance evidence are recorded in README.md and visual-communication.verification.md.

## Corrections log
- 2026-11-23: user clarified “unlicensed” means no license grant, not the Unlicense public-domain dedication.
- 2026-11-23: user: “It should open automatically the first time at least” and “It should be a standalone app, not a webpage in my default browser.” These replace the earlier manual-launch/browser-first decisions.
- 2026-11-23: user: “the GUI should be keyboard first. I'm extremely partial to keyboard experiences.”

## Licensing constraint
- No license grant for project code: no LICENSE file; npm metadata UNLICENSED; no Cargo license declaration. Existing third-party notices remain intact.

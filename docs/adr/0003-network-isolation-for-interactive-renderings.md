# Require a browser-enforced network boundary for interactive renderings

Interactive JavaScript must not contact external endpoints or companion controls. An exercised WebRTC probe sent UDP traffic despite `connect-src 'none'`, so CSP alone does not satisfy that contract: apply `Connection-Allowlist` to the opaque-origin rendering frame and require an engine supporting it. The desktop app now supplies Chromium 156 through Electron, rather than depending on the user's default browser. Keep companion controls outside the frame and test both HTTP and WebRTC boundaries.

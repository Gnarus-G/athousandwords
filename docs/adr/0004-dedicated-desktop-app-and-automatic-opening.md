# Publish into an automatically opened desktop app

The user tried the browser-first version and explicitly requested a standalone app and automatic first-use opening. Use a dedicated Electron window with a pinned Chromium runtime instead of delegating to the default browser; this adds a runtime dependency but removes browser chrome, extensions, and engine-version mismatch from the communication path. The local companion launches it when no desktop viewer is connected, coalesces startup requests, and reuses an existing instance; remote adapters never open windows on the remote machine.

`atw open` now opens or raises this application. `serve --no-open` is the explicit headless mode. Only the desktop viewer contributes render evidence in desktop mode, so a stale browser tab cannot suppress opening or overwrite the app's render acknowledgment. Launch requests and failures remain separate from actual rendering.

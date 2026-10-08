# Windmill browser extension

A Chrome (Manifest V3) extension that runs Windmill AI sessions in the browser side panel,
with tools that act on the active tab.

The side panel frames Windmill's own chat-only route, `/sessions/browser`, so the session
loop, history, backups and sign-in are the instance's own. The extension adds only the
browser tools: once the framed page completes a `postMessage` handshake, the session is offered
`browser_read_page`, `browser_screenshot`, `browser_click`, `browser_type` and
`browser_navigate`. The extension runs them on the active tab of the panel's window.
Drafts, previews and page navigation, which need the Windmill editor, are withheld in the
panel (`browserTools` in `AIChatManager`).

## Build and load

```bash
npm install
npm run build
```

Load `dist/` as an unpacked extension (`chrome://extensions`, Developer mode, Load
unpacked). Set the instance URL in the extension's options page, then click the toolbar
icon to open the panel.

## Guardrails

- Only the active tab of the panel's window is read or acted on.
- Reading and screenshots run without asking. Each click, type and navigation waits for the
  chat's own approval card, which lives in Windmill's frame where the page cannot reach it.
  YOLO auto-approval is unavailable in the panel.
- Approval runs in two steps. When the card opens, the extension pins the target (tab,
  document and element, the element held in the extension's isolated world) and describes
  it for the card. After approval it acts only on that target, and refuses if the tab is no
  longer active, the page navigated or reloaded, or the element changed.
- Both sides check the bridge's origins. The panel accepts messages only from its own frame
  at the configured instance origin. Windmill accepts them only from its parent window,
  when that parent is a `chrome-extension://` origin (`location.ancestorOrigins`).
- Page content is labelled untrusted in every tool result.

## Why `<all_urls>`

Chrome sends the instance's `SameSite=Lax` session cookie to a frame inside an extension
page only when the extension holds host permission for that instance. The browser tools
also need host permission for whatever site the active tab shows. Without the permission,
the framed chat loads signed out.

## Notes

- Sign-in methods that refuse to be framed (most SSO providers) should be completed in a
  normal tab first. The panel then shares that session.
- An instance that sets `CSP_POLICY` with a `frame-ancestors` directive must allow
  `chrome-extension://<extension id>`. Windmill sends no framing restriction by default.

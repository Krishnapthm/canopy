# 0025 - Sonner error toasts

Status: Accepted (2026-11)

## Context

The single global Alert error line could overwrite earlier failures. Agent failures
could remain visible only in daemon.log. Every error a user causes or can see must
reach them as a toast, never only in daemon.log.

## Decision

Add sonner to ADR 0022's approved web dependency list. Use the installed shadcn
Toaster once at the app root with its default neutral appearance. Sonner replaces
the single global Alert error line and local extension error alerts.

Sonner 2.0.8 declares the MIT license in web/node_modules/sonner/package.json.
Its installed size is approximately 192 KiB. The production build bundles it into
dist/assets/index-DDUc7LCk.js: 398.60 kB raw, 124,799 bytes gzip in the generated
sidecar. This is the whole entry chunk, not sonner alone. Total first-load JS is
160,994 bytes gzip, below the 163,840-byte budget.

Route failed requests through one persistent error-toast helper. Failed node
events use the session id, the first three lines of the plain-text reason, and
an Open action selecting that agent in the tree. Initial history is silent.
A stable reconnect info toast uses the default duration and is dismissed after
the connection and tree subscription recover.

## Consequences

Errors stay until dismissed; later errors no longer replace earlier ones.
Extension replies containing partial errors also produce toasts. New-agent
rejections use the same request path and preserve the daemon's message.
No new router or state library is needed. Browser layout, focus and accessibility
still require manual verification.

## Rejected

Handwritten toast state and controls would duplicate stacking, timers, focus and
accessibility behavior. The standard library or twenty lines cannot supply these
reliably. Sonner is shadcn's own toast component and is already installed.
Keeping the global Alert would preserve the lost-error problem.

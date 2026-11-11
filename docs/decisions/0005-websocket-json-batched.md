# 0005 - WebSocket, JSON text frames, batching, no compression

Status: Accepted (2026-11)

## Context

The browser needs a bidirectional channel with the lowest decode cost. Bytes are free on loopback.

## Decision

WebSocket with JSON text frames, server-side batching of one frame per 16 ms per client, and no `permessage-deflate`. Implemented in M5.

## Consequences

Batching matters far more than encoding. Compression would burn CPU to save bytes that cost nothing. Not built yet; nothing exists to verify against.

## Rejected

SSE: one-way, needs a second channel for commands. WebTransport: needs HTTP/3 and TLS certificates on localhost. MessagePack or protobuf: slower to decode in JS, extra dependency.

# 0021 - Solid JSX build plugin

Status: Superseded by [0022](0022-react-shadcn-web.md) (2026-11)

## Context

ADR 0012 approves Solid and Vite but omits their JSX compiler integration.

## Decision

Use the pre-installed vite-plugin-solid (MIT, 2.11.16) as a development dependency.
It replaces a custom Vite transform and invokes the Solid compiler to generate fine
grained DOM bindings. Its installed distribution is approximately 140 KB; it adds
no runtime code to the browser bundle. The standard library cannot compile JSX and
twenty lines of custom code cannot implement Solid's reactive JSX transform.

## Consequences

The build uses the official Solid integration. No new runtime dependency is added.

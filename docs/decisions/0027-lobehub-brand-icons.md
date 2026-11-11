# 0027 - LobeHub brand icons

Status: Accepted (2026-11)

## Context

Generic lucide Bot and Code icons do not identify the vendors in the web control
room. Third-party apps and services need recognizable monochrome brand marks.
The user chose LobeHub as the single source for those marks.

## Decision

Approve the already installed @lobehub/icons 6.15.0, licensed MIT, for brand icons.
It occupies about 32 MB on disk; only imported Mono files enter the bundle.
Keep lucide for controls, status and other harnesses. Map the binary and display
names of the two harnesses Canopy runs, Claude Code and Codex, to their marks.

Import each brand directly from its es/<Name>/components/Mono.js path. The package
exports map resolves this to the .mjs component and .d.mts types. These components
use React and currentColor, preserving the neutral theme.

Keep web/.npmrc legacy-peer-deps=true so npm does not install the peer @lobehub/ui.
The package root and brand indexes pull in that heavy UI kit through Avatar and
Combine and do not resolve without it. The Makefile check rejects imports other
than per-brand Mono.js paths.

## Consequences

The shared harnessIcon helper supplies brand marks to the tree, session header,
Running here, new-agent selector and empty-state harness list. Decorative marks
are hidden from assistive technology and use size-4 beside labels.

First-load JS is 163,754 of 163,840 bytes gzip with the two marks. OpenCode,
Copilot and Gemini marks would add about 2.4 KB and break the budget, so those
harnesses, which Canopy cannot run yet, keep the generic icon until it can. The
remaining budget headroom is 86 bytes: the next first-load addition must recover space.

## Rejected

- Package-root or brand-index imports and @lobehub/ui: they pull in a heavy UI kit.
- Hand-copied SVG paths: the user chose LobeHub as the single source.
- Color variants: they conflict with the neutral theme.
- Standard-library or handwritten generic icons: they do not supply the chosen
  vendor artwork and would retain the identification problem.

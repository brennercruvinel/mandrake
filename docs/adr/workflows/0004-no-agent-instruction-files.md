---
id: ADR-0004
title: No agent instruction files
kind: decision
status: accepted
category: WORKFLOWS
date: 2026-10-25
last-updated: 2026-10-25
tags: [contributors]
supersedes: []
superseded-by: null
related: ["#13", "#15", CONTRIBUTING.md]
---

# ADR-0004: No agent instruction files

## Context

Many repositories carry files written for coding agents: `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.cursorrules`, `.github/copilot-instructions.md`. Mandrake is a tool that trims what Claude Code puts in front of the model, and contributors arrive with agents of their own. A second set of instructions next to the docs drifts from them, and it is loaded into every agent session in the repository whether it helps or not.

## Considered options

### One instruction file per agent

Each agent finds its file, but the same rules are written several times, and they go stale where nobody reads them.

### No instruction files

Agents read what people read: the code, `docs/`, the ADRs, the changelog and `CONTRIBUTING.md`.

## Decision

No agent instruction files are tracked. The code, the docs, the ADRs and the changelog are the context.

## Consequences

- One source for people and agents, and nothing extra loaded into a session by default.
- What an agent needs to know has to be written where a person would look for it.
- Follow-up: the gate fails when one of those files is tracked (#15).

## References

- #13, #15

---
id: ADR-0001
title: Decision records and changelog fragments
kind: decision
status: accepted
category: WORKFLOWS
date: 2026-10-25
last-updated: 2026-10-25
tags: [contributors]
supersedes: []
superseded-by: null
related: ["#13", "#14", docs/plan-v0.md]
---

# ADR-0001: Decision records and changelog fragments

## Context

The v0 plan (`docs/plan-v0.md`, 2026-10-24) said "internal use, no ADR": `docs/` explained how Mandrake works, and the decisions lived in that plan and in chat. Two contributors joined in #3 the next day. They cannot read the chat, the plan mixes decisions with a task list, and nothing told them what changed between two pulls.

## Considered options

### Keep the plan as the record

Nothing new to maintain, but the plan is a task list of one release: a decision is hard to find in it, and it stops being read once the release is done.

### One changelog edited by hand

Easy to read, but every pull request edits the same lines of one file, so two open pull requests conflict.

### ADRs plus changelog fragments, as urna does

A decision gets its own short file with the trade-offs; a change gets one line in `docs/news/`, and towncrier turns the fragments into the release section. No two pull requests touch the same line.

## Decision

ADRs in `docs/adr/<category>/` and towncrier fragments in `docs/news/`, in urna's format, with five categories to start (`structure`, `interface`, `promptcut`, `installer`, `workflows`) and urna's five change types. This replaces the "no ADR" line of the v0 plan.

## Consequences

- A contributor finds why something is the way it is without asking, and sees what changed in `docs/CHANGELOG`.
- Each pull request carries a fragment or the `skip-news` label: one more file per change.
- Follow-up: `tool/fullcheck.sh` runs `towncrier check` (#15).

## References

- #13, #14; urna's `docs/adr/README.md` and `[tool.towncrier]`

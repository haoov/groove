# Agent

## Multi-agent

- per session: status, asks, time
- approvals: approve, review, refuse
- auto-approve

## MCP tools

- reads
- writes: ask, or auto when auto-approve is on
- scoped to the session
- harness description

## Activity

- hooks
- status
- timeline

## Core prompt

- session identity
- rules

## Skills

- core skills
- user skills
- actions: list, read, save, delete, switch

## Shared skills

- the team's plugins, from the shared repo
- each skill switched on one by one

## Knowledge

- the shared repo's facts, named in the core prompt

## Routines

- kinds: bound, standalone, action
- triggers: daily, app events, the Run button
- only on sessions with auto-approve on; never the selected one; once until selected
- cap, pause all
- the run trail on the feed

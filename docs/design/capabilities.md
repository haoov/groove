# Capabilities

What Groove does, at the top level. Each capability lists its features, and its
implementation file under `capabilities/` says how they are made.

A tool-set item is a feature the user can name. Never a surface, never a module.

| Capability | Surface | Implementation |
|---|---|---|
| Tasks | board | [capabilities/tasks.md](capabilities/tasks.md) |
| Agent | rail | [capabilities/agent.md](capabilities/agent.md) |
| Sessions | session: overview and agent pane | [capabilities/sessions.md](capabilities/sessions.md) |
| Workspace | session: work region | [capabilities/workspace.md](capabilities/workspace.md) |
| Config | settings | [capabilities/config.md](capabilities/config.md) |

## Tasks

Pull and file your tasks from providers and manage them as sessions.

- Multi providers
- Tasks management

## Agent

Manage a multi-agent system, with activity and actions approvals.

- Multi-agent
- MCP tools
- Activity
- Core prompt
- Skills
  
## Sessions

Centralized work for a task, with Agent session repos/worktrees management and workspace.

- Repos management
- Worktrees management
- Overview
- Agent session

## Workspace

Code review and editing.

- Diff
- Annotations
- Editor
- Terminal
- Git
- Forge

## Config

Set Groove up and keep its preferences.

- Setup
- Providers
- Appearance
- Preferences

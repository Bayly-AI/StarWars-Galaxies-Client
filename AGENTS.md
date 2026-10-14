# AGENTS.md — Client

> Role: **Client** · Member of Enterprise Agentic Platform

## CR-CLI-ENTRY-001: Start with the CLI (CRITICAL — Org-Wide)

1. **Start with the CLI (CRITICAL ENTRY GATE)**: When receiving ANY request or starting any task, agents **MUST ALWAYS START WITH THE OPERATOR CLI (`hath0r`)** rather than inventing ad-hoc scripts.
2. **Missing Capability Offer**: If the required connection, MCP, workflow, bot, or factory does not exist in `hath0r`, offer to register the missing capability.
3. **Docs before code**: Require procedure/playbook/runbook before scaffolding implementation.

## Promotion Path (CR-BAI-001)

```text
local → development → testing → staging → master (Production)
```

## Branch Governance (cr-branch-gov-001)

- Issue first: create an issue before cutting a branch.
- Naming: `feature|bugfix|enhancement|chore/<issue-number>-slug`
- Base PR target: `development`

## AgentGraph Substrate

This repository is governed by the Hath0r AgentGraph substrate. Dynamic rule retrieval, role RBAC, and policy graphs are stored under `.hath0r/agentgraph/`.
- Query status: `hath0r agentgraph status`
- Query rules: `hath0r agentgraph query "<topic>"`
- Route role: `hath0r agentgraph route --role <role>`
- Validate rules: `hath0r agentgraph validate`

## Hyper Context Pointers
- **Contracts Subsystem**: `/Users/raybayly/Development/BAI/GAMES/SWG/Client/contracts/AGENTS.md`

## Hyper Context Pointers
- **Docs Subsystem**: `/Users/raybayly/Development/BAI/GAMES/SWG/Client/docs/AGENTS.md`

## Hyper Context Pointers
- **Cfg Subsystem**: `/Users/raybayly/Development/BAI/GAMES/SWG/Client/cfg/AGENTS.md`

## Hyper Context Pointers
- **Tests Subsystem**: `tests/AGENTS.md`

# Plan: Port GenericAgent's Best Ideas into RimFrost

**Goal:** Make rimfrost learn from its sessions and get smarter over time, inspired by GenericAgent's self-evolving memory system.

**Source of inspiration:** `lsdefine/GenericAgent` — specifically its `memory/` system, `update_working_checkpoint` tool, and failure escalation prompt patterns.

---

## Phase 1 — Failure Escalation in System Prompt
**Scope:** `runtime/src/prompt.rs`
**Effort:** Small (prompt text only, no new code paths)

Add a structured failure-handling section to `get_simple_doing_tasks_section()`:

```
- On first failure: read the error output carefully and understand the root cause before retrying.
- On second failure: probe the environment (check file state, dependencies, versions) to gather new information.
- On third failure: step back, analyze what you have learned, and switch to a fundamentally different approach — or ask the user. Never repeat the same action without new information.
```

This is GenericAgent's "1st fail → read error; 2nd → probe env; 3rd → switch approach or ask" pattern. Zero risk, immediate value.

**Files:**
- `rust/crates/runtime/src/prompt.rs` — modify `get_simple_doing_tasks_section()`

---

## Phase 2 — Working Memory Checkpoint Tool
**Scope:** `tools/src/lib.rs`, `runtime/src/session.rs`
**Effort:** Medium (new tool, session state extension)

Add a `WorkingCheckpoint` tool that lets the agent maintain a short scratchpad (~200 tokens) that persists across turns within a session. GenericAgent's `update_working_checkpoint` prevents context drift in long tasks — the agent writes "where I am, what matters, what's next" and it gets injected into every subsequent turn.

### Design

**New tool: `WorkingCheckpoint`**
```json
{
  "name": "WorkingCheckpoint",
  "description": "Update the working memory scratchpad for this session. Content is injected into every subsequent turn to prevent context drift during long tasks. Use when: (1) starting a complex multi-step task, (2) before switching subtasks, (3) after failures to record what you learned. Keep under 200 tokens.",
  "parameters": {
    "key_info": { "type": "string", "description": "Compressed state: current goal, key findings, pitfalls, file paths, progress, next steps." }
  }
}
```

**Storage:** Add `working_checkpoint: Option<String>` to the `Session` struct. Persisted in the session JSONL alongside other session metadata.

**Injection:** In `ConversationRuntime::run_turn()`, if `session.working_checkpoint` is `Some`, prepend it as a `<system-reminder>` block to the user message:
```
<system-reminder>Working memory checkpoint:
{checkpoint_content}
</system-reminder>
```

**Files:**
- `rust/crates/tools/src/lib.rs` — add `WorkingCheckpoint` tool spec + dispatch + execution
- `rust/crates/runtime/src/session.rs` — add `working_checkpoint` field to `Session`
- `rust/crates/runtime/src/conversation.rs` — inject checkpoint into turn messages

---

## Phase 3 — Cross-Session Memory (Long-Term Learning)
**Scope:** `tools/src/lib.rs`, `runtime/src/prompt.rs`, new module `runtime/src/memory.rs`
**Effort:** Large (new subsystem, prompt integration, file I/O)

This is the big one. GenericAgent's killer feature: the agent writes learnings to disk, and they're loaded into the system prompt on every future session. Over time, the agent accumulates project-specific knowledge, SOPs, and user preferences.

### Design

**Memory directory:** `~/.rimfrost/memory/` (user-level, cross-project) and `.rimfrost/memory/` (project-level)

**Memory files:**
```
~/.rimfrost/memory/
├── global.md          # User preferences, environment facts, general learnings
├── insights.md        # Structured summary (distilled periodically)
└── skills/            # SOPs the agent writes after learning how to do something
    ├── docker-setup.md
    └── rust-ci-fix.md

.rimfrost/memory/
├── project.md         # Project-specific facts and patterns
└── skills/
    └── deploy-staging.md
```

**New tool: `MemoryWrite`**
```json
{
  "name": "MemoryWrite",
  "description": "Save a learning, fact, or SOP to long-term memory. Loaded into future sessions automatically. Use when: discovering environment facts, user preferences, reusable procedures, or lessons from failures.",
  "parameters": {
    "scope": { "type": "string", "enum": ["user", "project"] },
    "category": { "type": "string", "enum": ["fact", "preference", "skill"] },
    "title": { "type": "string" },
    "content": { "type": "string" }
  }
}
```

**New tool: `MemoryRead`**
```json
{
  "name": "MemoryRead",
  "description": "Search long-term memory for relevant learnings, facts, or SOPs.",
  "parameters": {
    "query": { "type": "string" },
    "scope": { "type": "string", "enum": ["user", "project", "all"] }
  }
}
```

**Prompt injection:** In `SystemPromptBuilder::build()`, after the environment section, load and inject memory:
1. Read `~/.rimfrost/memory/global.md` and `.rimfrost/memory/project.md`
2. Truncate to a budget (e.g., 2000 chars total)
3. Inject as a `# Long-term memory` section

Skills are NOT auto-loaded (too much context). Instead, the agent uses `MemoryRead` to search when relevant.

**Files:**
- `rust/crates/runtime/src/memory.rs` — new module: read/write/search memory files
- `rust/crates/runtime/src/lib.rs` — export memory module
- `rust/crates/runtime/src/prompt.rs` — inject memory into system prompt
- `rust/crates/tools/src/lib.rs` — add `MemoryWrite` and `MemoryRead` tool specs + dispatch

---

## Execution Order

| Phase | What | Risk | Depends on |
|-------|------|------|------------|
| 1 | Failure escalation prompt | None — text-only change | Nothing |
| 2 | Working memory checkpoint | Low — new tool, session field | Nothing |
| 3 | Cross-session memory | Medium — new subsystem, file I/O | Phase 1 (for prompt patterns) |

Phase 1 and 2 can be done in parallel. Phase 3 builds on the prompt infrastructure from Phase 1.

---

## What we're NOT porting

- **Multi-frontend** (WeChat/Telegram/Qt/desktop pet) — rimfrost is CLI-first, this is a different product direction
- **Browser automation** (TMWebDriver/CDP bridge) — rimfrost already has WebSearch/WebFetch, full browser control is a separate project
- **Reflect mode** (file-watcher triggers) — interesting but niche, can be added later as a plugin
- **`ask_user` tool** — rimfrost already has `AskUserQuestion` / `SendUserMessage`
- **Mixin sessions** (multi-model routing) — rimfrost already has provider fallback config

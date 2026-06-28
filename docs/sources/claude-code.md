# Claude Code Source

Meridian reads Claude Code session transcripts directly from `~/.claude/projects/` with no export or configuration needed.

## How it works

Claude Code stores sessions at:
```
~/.claude/projects/<project-hash>/<session-uuid>.jsonl
```

Each session file is a JSONL stream with records of type `user`, `assistant`, `ai-title`, and `mode`.

Meridian extracts:
- **Title**: the `ai-title` record if present, otherwise the first user message (up to 120 characters)
- **Tokens**: `input_tokens + cache_creation_input_tokens` (input), `output_tokens` (output) summed across all assistant turns. `cache_read_input_tokens` is excluded — it's ~10x cheaper and distorts the cost signal.
- **Project path**: decoded from the directory name (`-Users-michael-dev` → `/Users/michael/dev`)
- **Timestamp**: the first user turn timestamp

Sessions with no user turns (e.g., empty or system-only files) are skipped.

## Usage

```bash
# Auto-detect (uses ~/.claude/projects/ if found)
meridian analyze

# Explicit
meridian analyze --source claude-code

# Custom location
meridian analyze --source claude-code --path /path/to/.claude/projects
```

Or configure in `meridian.toml`:
```toml
[source]
claude_code_dir = "~/.claude/projects"
```

## Setup wizard

```bash
meridian setup
```

If `~/.claude/projects/` exists, the wizard offers it as the default source and pre-selects it.

## Token cost reference

| Token type | Cost signal |
|---|---|
| `input_tokens` | Included (prompt tokens, direct cost) |
| `cache_creation_input_tokens` | Included (written to cache, ~1.25x cost) |
| `cache_read_input_tokens` | **Excluded** (read from cache, ~0.1x cost) |
| `output_tokens` | Included, weighted 3x (generation cost) |

The effort score formula: `(input / 1000) + (output / 1000 * 3)`.

## Cross-project analysis

Meridian reads all projects in the directory. Each project directory is decoded to a path and stored in the `metadata.project` field. You can see which project a session came from in JSON export:

```bash
meridian analyze --source claude-code --format json | jq '.by_category'
```

## Limitations

- Sessions must have at least one `user` turn to be included
- Title extraction depends on Claude Code generating an `ai-title` record; some short sessions may only get the first user message as the title
- Token counts reflect what Claude Code records — multi-turn sessions accumulate naturally

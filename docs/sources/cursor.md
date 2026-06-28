# Cursor Integration

Cursor does not expose session logs in a machine-readable format by default. There are three approaches depending on your workflow.

## Option 1: Manual log (recommended for quick start)

Keep a simple log file and add entries as you work. Meridian reads any JSONL file — it doesn't need to be automated.

Create `sessions.jsonl`:
```json
{"title": "Refactor authentication module", "tokens_input": 12000, "tokens_output": 4500, "timestamp": "2026-06-10T14:00:00Z"}
{"title": "Add payment webhook handler", "tokens_input": 8000, "tokens_output": 3200, "timestamp": "2026-06-11T09:30:00Z"}
{"title": "Write API documentation", "tokens_input": 5000, "tokens_output": 2100, "timestamp": "2026-06-11T15:00:00Z"}
```

Then:
```bash
meridian setup   # pick "Generic JSONL" source, point at sessions.jsonl
meridian analyze
```

See [standalone.md](standalone.md) for the full JSONL field reference.

## Option 2: .cursorrules export hook

Add a rule to your `.cursorrules` file that asks Cursor to log sessions:

```
# Meridian tracking
After completing any significant task, append to sessions.jsonl:
{"title": "<brief task description>", "timestamp": "<ISO8601>", "tokens_input": <approximate>, "tokens_output": <approximate>}
```

This relies on the model following instructions consistently — useful as a lightweight habit.

## Option 3: Cursor API (future)

Cursor is adding usage APIs. Once available, a native Cursor adapter will be added to `meridian-ingest`. Watch the [GitHub repo](https://github.com/Trusted-Autonomy/meridian) for updates.

## Configuring meridian.toml for Cursor

```toml
[source]
jsonl = "sessions.jsonl"   # path to your log file

[[kpis]]
id = "eng_velocity"
label = "Engineering Velocity"
description = "Ship quality features faster"
weight = 1.0

# ... add your KPIs
```

## Tips for accurate categorization

The keyword scorer classifies sessions by title. Write descriptive task names:
- "fix auth bug" → categorizes to Security
- "implement payment webhook" → categorizes to Code
- "write onboarding guide" → categorizes to Documentation

Generic titles like "help with code" or "task" classify poorly. The more specific the title, the better the report.

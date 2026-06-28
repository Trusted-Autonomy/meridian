# Generic JSONL Format

Meridian can ingest effort records from any tool via a simple JSONL file. One JSON object per line.

## Minimal record

```json
{"title": "Implement user authentication"}
```

Only `title` is required. Everything else has sensible defaults.

## Full record

```json
{
  "id": "session-abc123",
  "title": "Implement OAuth2 login flow",
  "timestamp": "2026-06-10T14:32:00Z",
  "tokens_input": 12500,
  "tokens_output": 4200,
  "seconds": null,
  "source": "cursor",
  "phase": "v0.3.0"
}
```

## Field reference

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | No | Unique identifier. Auto-generated if omitted. |
| `title` | string | Yes | Short description of the work. Used for classification. |
| `timestamp` | ISO 8601 | No | When the session occurred. Defaults to the current time if missing. |
| `tokens_input` | integer | No | Input tokens consumed. |
| `tokens_output` | integer | No | Output tokens generated. |
| `seconds` | integer | No | Time spent in seconds (for non-token-based tools). |
| `source` | string | No | Label for the originating tool (e.g., "cursor", "copilot"). |
| `phase` | string | No | Version or sprint tag for filtering (e.g., "v0.3.0", "Q3-2026"). |

If both tokens and seconds are present, tokens take precedence for the effort score.

## Effort score formula

- **Tokens**: `(tokens_input / 1000) + (tokens_output / 1000 * 3)` — output weighted 3x because it's ~3x more expensive to generate than to read.
- **Seconds**: raw value (no normalization).
- **Mixed files**: records with tokens use the token formula; records with only seconds use the seconds value.

## Example: Copilot / GitHub Copilot

GitHub Copilot does not expose session logs publicly. Export your work log manually or via an IDE extension:

```json
{"title": "Add database migration for user profiles", "tokens_input": 8000, "tokens_output": 2500, "source": "copilot", "timestamp": "2026-06-11T10:00:00Z"}
{"title": "Write unit tests for auth service", "tokens_input": 6000, "tokens_output": 3100, "source": "copilot", "timestamp": "2026-06-11T11:30:00Z"}
```

## Example: Any chat-based AI workflow

Track your prompts in a nightly cron script or manually:

```bash
# Append to sessions.jsonl after each session
echo '{"title":"Investigate payment timeout bug","tokens_input":9000,"tokens_output":3500,"timestamp":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"}' >> sessions.jsonl
```

## Using the JSONL source

```bash
meridian analyze --source jsonl --path sessions.jsonl

# Or configure in meridian.toml:
[source]
jsonl = "sessions.jsonl"
```

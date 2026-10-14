# Tokenomics Validation and Per-Epoch Cost Scaling

**Status:** proposed note, 2026-10-14, at the owner's direction. Design only; nothing here is built. It accompanies the Claude Code ingester fix (dedupe by `message.id`, count cache-read) approved the same day.

## Why this exists

Meridian turns token usage into "effort points" with a fixed formula (`docs/quickstart.md`):

    effort = input/1000 + output/1000 * 3

Three measured problems, all from one real Claude Code session (`98a823ac`, 1,259 unique API turns):

| Problem | Measured | Effect on effort |
|---|---|---|
| Ingester counts every streamed row, not unique `message.id` | 2.49x input, 2.69x output overcount | Effort inflated about 2.5x to 2.7x |
| Cache-read tokens are not counted at all | 603.0M cache-read tokens | The largest cost line is invisible. Cache read was 63.8% of modelled cost, cache write 30.9%, output 5.3% |
| The weights (1 : 3) are constants | n/a | They encode one vendor price ratio at one moment; a price change or a model change silently rescales every historical and future number |

The first two are bugs and are fixed by the ingester change. The third is structural and is what this note is about: **a fixed weighting makes effort numbers incomparable across time**, and nothing in Meridian would notice.

Dollar totals depend entirely on which rate sheet is applied, while the shares do not. The same 1,259 turns price to $283 at Sonnet-class list rates ($3 / $15 / $0.30 / $6 per million tokens for input / output / cache read / 1h cache write), $472 at $5 / $25 / $0.50 / $10, and $1,417 at Opus-class $15 / $75 / $1.50 / $30, a 5x spread, with identical shares (63.8% cache read, 30.9% cache write, 5.3% output) in every case. That is the concrete reason the rate set must be a named, versioned epoch and not an unstated constant.


## Requirements (from the owner)

1. **Stronger validation on tokenomics.**
2. **Or a cost metric that everything is scaled against, per epoch.**

These are two halves of one design: validate what comes in, and make the unit of account explicit and versioned so a change of unit is visible instead of silent.

## Design

### 1. A cost metric per epoch

- An **epoch** is a period during which one set of cost weights is in force. It has an id, a start timestamp, and a weights record: price per million tokens for each token class (input, output, cache read, cache write 5m, cache write 1h), per model family.
- Every ingested record is stamped with the epoch id that applied at its timestamp. The epoch table is data (a TOML file next to `meridian.toml`), not code, matching how KPIs are configured.
- **The reference metric** is "dollars at epoch E list price" or, when a runtime reports its own cost, the runtime-reported cost. Effort points become a derived, scaled view: `points = cost_in_epoch / reference_unit_cost`, where the reference unit is declared per epoch. Changing the weights starts a **new epoch**; it never edits an old one, so history stays reproducible.
- Reports can show a series in its own epoch's units or rescaled to a chosen epoch, and say which. A report that mixes epochs without rescaling is refused, not rendered.

Why not keep points and just fix the weights: because the correct weights change (cache pricing, new models, the 1h cache-write class that Claude 5 introduces), and a single global formula cannot represent that without rewriting history.

### 2. Validation at ingest

Validation runs per source and per file, and failures are reported, never swallowed. Each check has a name so it can be asserted in tests and shown in `meridian` output.

| Check | What it catches | Action |
|---|---|---|
| Duplicate id | The same `message.id` counted more than once (the 2.5x bug) | Dedupe, count how many were dropped, report the count |
| Class completeness | Token classes present in the source but dropped by the ingester (cache read today) | Fail loudly if a known class is seen and has no weight |
| Reconciliation | Sum of per-record tokens differs from a session total the runtime reports (TA goals and OMP both report totals) | Warn above a configured tolerance, show both numbers |
| Cost cross-check | Computed cost versus runtime-reported cost, where both exist | Warn when they diverge beyond tolerance; this is how stale price tables get noticed. Measured on TA goals: table-estimated rates were wrong in both directions, so this check would have fired |
| Plausibility bounds | Negative counts, output larger than any model's maximum, context above the model window, a timestamp outside the epoch range | Reject the record, count it |
| Unknown model or unpriced class | A model with no weights in the active epoch | Refuse to price it; do not fall back to a default rate silently |
| Epoch gap | A record whose timestamp falls in no epoch | Refuse and name the gap |
| Scale drift | A source's median cost per record moving by more than a configured factor between consecutive periods with no epoch change | Warn: either behaviour changed or the weights are stale |

The ingest summary always prints: records read, unique after dedupe, rejected by reason, priced, unpriced, and the epoch ids used. A run that rejects or leaves anything unpriced exits non-zero unless explicitly allowed.

### 3. Interaction with other work

- **TrustedAutonomy `ta cost report` (cost spec S2)** emits Meridian JSONL. Its records should carry the model, every token class separately, and the runtime-reported cost when present, so Meridian can do the cost cross-check instead of receiving a pre-collapsed number. This is the contract that makes check 4 possible.
- **Goal start state and golden-set replay** compare runs on cost. They need a stable unit, which is exactly the epoch's reference metric; two replays in different epochs must be rescaled before they are compared.
- Existing history (before epochs) is migrated into a single epoch 0 that records the old `1 : 3` weights and is marked `legacy: cache classes not priced`, so old numbers stay reproducible and visibly different from new ones.

## Open questions

1. What is the reference unit: dollars at list price, or a normalised "points" unit fixed at epoch 0? Dollars are the honest unit but need rates kept current; points are stable but need the rescaling to be correct. Recommendation: store dollars and tokens by class, derive points on read.
2. Tolerances for reconciliation and cost cross-check. Set from measured data once the fixed ingester runs over the real corpus, not guessed.
3. Where epoch weights come from. Hand-edited TOML with a review step is the safe start; scraping vendor price pages is not recommended (the same table rates were already wrong in both directions here).

## Out of scope

Billing-grade accuracy, multi-currency, per-user chargeback.

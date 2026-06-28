# Example: `meridian report --since 7d`

Same company: Stackwise Inc., week of Jun 21–28, 2026.
12 aligned sessions + 3 unaligned. 1.58M tokens total.

The session table shows each session with its effort, consensus panel score,
and weighted KPI alignment score. The `▸` line immediately below each session
expands the per-KPI breakdown, sorted by alignment score descending. The
PANEL AVERAGES section at the bottom shows aggregate per-panelist scores for
the period.

---

```
$ meridian report --since 7d

 Report  2026-06-21 -> 2026-06-28   (15 sessions)

 Session                                  Effort  Consensus  KPI-Align  !
 ─────────────────────────────────────────────────────────────────────────
 Implement SSO with Okta (enterprise)      340K     0.81       0.79
     ▸ Security: 0.91  Enterprise: 0.82  Dashboard: 0.71  Engineering: 0.68  Revenue: 0.62
 Implement audit log, permission changes   195K     0.78       0.75
     ▸ Security: 0.92  Enterprise: 0.78  Engineering: 0.61  Dashboard: 0.58  Revenue: 0.48
 Design enterprise admin onboarding flow   220K     0.76       0.73
     ▸ Enterprise: 0.85  Revenue: 0.79  Engineering: 0.68  Dashboard: 0.60  Security: 0.58
 Set up DataDog APM for production         160K     0.70       0.66
     ▸ Dashboard: 0.88  Engineering: 0.81  Security: 0.70  Enterprise: 0.62  Revenue: 0.44
 Write SOC 2 access control docs            95K     0.68       0.71
     ▸ Security: 0.90  Enterprise: 0.76  Engineering: 0.58  Dashboard: 0.45  Revenue: 0.38
 Refactor GitHub connector rate limiting   145K     0.64       0.58
     ▸ Engineering: 0.82  Dashboard: 0.75  Enterprise: 0.55  Security: 0.48  Revenue: 0.30
 Fix N+1 query bug in dashboard API        180K     0.60       0.55       !
     ▸ Dashboard: 0.84  Engineering: 0.80  Enterprise: 0.52  Security: 0.44  Revenue: 0.30
 Debug Stripe webhook retry logic           88K     0.55       0.49       !
     ▸ Engineering: 0.72  Revenue: 0.62  Dashboard: 0.58  Enterprise: 0.44  Security: 0.40
 Competitive analysis vs. Jellyfish         55K     0.67       0.64
     ▸ Revenue: 0.82  Dashboard: 0.75  Enterprise: 0.65  Engineering: 0.52  Security: 0.40
 Draft pricing page copy, annual plans      42K     0.59       0.61
     ▸ Revenue: 0.80  Enterprise: 0.62  Security: 0.35  Dashboard: 0.30  Engineering: 0.28
 Fix flaky test in CI pipeline              38K     0.44       0.40
     ▸ Engineering: 0.72  Dashboard: 0.55  Security: 0.42  Enterprise: 0.38  Revenue: 0.20
 Weekly planning standup notes              15K     0.46       0.42
     ▸ Enterprise: 0.58  Engineering: 0.52  Revenue: 0.48  Dashboard: 0.38  Security: 0.32
 What does this error mean?                  8K     0.22       0.18
     ▸ Engineering: 0.32  Dashboard: 0.28  Security: 0.18  Enterprise: 0.12  Revenue: 0.08
 Help me write a LinkedIn post              12K     0.18       0.14
     ▸ Revenue: 0.28  Enterprise: 0.18  Engineering: 0.12  Dashboard: 0.10  Security: 0.08
 Debug random 500 error in staging          22K     0.38       0.32
     ▸ Dashboard: 0.62  Engineering: 0.58  Security: 0.35  Enterprise: 0.28  Revenue: 0.15
 ─────────────────────────────────────────────────────────────────────────
 Period average                               -      0.60       0.56

 PANEL AVERAGES
   CEO                   0.71
   CFO                   0.63
   Head Of Product       0.68
   Tech Director         0.74
   Lead Security Eng     0.81
   Head Of Marketing     0.44
   Head Of Sales         0.59
   Consensus             0.66   Dissent 0.13

 High dissent (>0.20):
   "Fix N+1 query bug in dashboard API"
     champion: Tech Director 0.82  skeptic: Head Of Sales 0.47
   "Debug Stripe webhook retry logic"
     champion: Tech Director 0.72  skeptic: CEO 0.48

```

---

## Reading the output

**Consensus** is the weighted-average score across all panelists for that session.
A high consensus means most panelists agree the work is aligned.

**KPI-Align** is the weighted-average alignment across all your configured KPIs,
using KPI weights from `meridian.toml`. A session can have high consensus but
moderate KPI-Align if the work is valuable but doesn't map cleanly to any specific
KPI description.

**`▸` KPI breakdown** shows per-KPI scores sorted by alignment descending. This
tells you *which* KPIs a session serves. In the example above:
- "Implement SSO with Okta" serves Security & Compliance (0.91) and Enterprise
  Readiness (0.82) most strongly — exactly right.
- "Draft pricing page copy" serves Revenue Conversion (0.80) strongly but
  Engineering Velocity (0.28) barely at all — expected, not a problem.

**`!` (High dissent)** means panelists disagree by >0.20 standard deviation.
The "Fix N+1 query bug" dissent reflects a real tension: the Tech Director
correctly reads it as a critical performance fix; the Head of Sales sees no
direct connection to the enterprise pipeline. Labeling it under "Dashboard
Performance" KPI would close the gap — the latency improvement directly
addresses the competitive gap with Jellyfish raised in pilot calls.

**PANEL AVERAGES** shows aggregate per-panelist scores for the period.
Head of Marketing at 0.44 reflects 0 sessions this week on growth or
content work — that's a real signal, not a scoring artifact.

## Getting more detail

```
# Full data including all per-KPI scores and raw panel scores
meridian report --since 7d --format json

# Spreadsheet-friendly output for historical analysis
meridian report --since 30d --format csv > june.csv

# Focus on a specific window
meridian report --since 2026-06-01
```

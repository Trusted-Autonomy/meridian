# Example: `meridian pm init`

Same company: Stackwise Inc. wants richer KPIs with real metrics, measurement sources,
and targets. They run `meridian pm init` to interview the virtual PM, get Claude to
generate structured KPIs, and then run them through a 7-panelist red team before writing
meridian.toml.

---

```
$ meridian pm init

Meridian Virtual PM — KPI Generation
──────────────────────────────────────

Step 1: Tell me about your priorities

? What is your team trying to accomplish? Be as specific or vague as you like.
> We need to close our first enterprise accounts — three companies in pilot right now.
  They keep asking about SSO, audit logs, and SOC 2. We also need to keep engineering
  velocity up while we do all this compliance work. And we're losing deals to Jellyfish
  because our dashboard is slower than theirs.

? Any constraints, hard deadlines, or pressures?
> SOC 2 Type I audit is booked for December 1st. We have 8 engineers total.
  Series A runway is 18 months — we need to show ARR growth by month 12.

? Team size (approximate)?
> 25

? Industry or domain?
> B2B SaaS developer tools

  Generating KPIs and metrics...  ████████████████ done (3.2s)

Step 2: Draft KPIs

  4 KPIs generated with 14 metrics total.

  ┌─────────────────────────────────────────────────────────────────────────┐
  │ 1. Enterprise Readiness                                        weight 1.5│
  │    Ship the features enterprise pilots need to convert.                  │
  │                                                                          │
  │    Metrics:                                                              │
  │    • pilot_feature_completion — % of pilot-requested features shipped    │
  │      target: >80%   unit: percent  source: linear    frequency: weekly   │
  │    • soc2_control_coverage — Controls documented + evidence collected    │
  │      target: 100%   unit: percent  source: manual    frequency: weekly   │
  │    • sso_audit_log_availability — SSO + audit log live for all pilots    │
  │      target: 3/3    unit: count    source: manual    frequency: weekly   │
  ├─────────────────────────────────────────────────────────────────────────┤
  │ 2. Engineering Velocity                                        weight 1.2│
  │    Keep shipping product features at pace despite compliance overhead.   │
  │                                                                          │
  │    Metrics:                                                              │
  │    • mean_time_to_merge — Average hours from PR open to merge            │
  │      target: <24h   unit: hours    source: git        frequency: weekly  │
  │    • compliance_overhead_ratio — % agent sessions on compliance vs. prod │
  │      target: <40%   unit: percent  source: ta_velocity frequency: weekly │
  │    • p95_review_time — 95th percentile PR review cycle time              │
  │      target: <48h   unit: hours    source: git        frequency: weekly  │
  ├─────────────────────────────────────────────────────────────────────────┤
  │ 3. Dashboard Performance                                       weight 1.1│
  │    Close the performance gap with Jellyfish.                             │
  │                                                                          │
  │    Metrics:                                                              │
  │    • p95_api_latency — 95th percentile API response time                 │
  │      target: <300ms  unit: ms      source: datadog   frequency: daily    │
  │    • dashboard_load_time — Time to interactive on main dashboard         │
  │      target: <2s     unit: seconds source: datadog   frequency: daily    │
  ├─────────────────────────────────────────────────────────────────────────┤
  │ 4. Revenue Conversion                                          weight 1.3│
  │    Convert pilots to paid annual contracts.                              │
  │                                                                          │
  │    Metrics:                                                              │
  │    • pilot_to_paid_conversion — Pilots that convert to annual contracts  │
  │      target: >2/3   unit: count   source: manual     frequency: monthly  │
  │    • arr_pipeline — Total ARR value in active enterprise pipeline        │
  │      target: >$400k  unit: dollars source: manual    frequency: monthly  │
  │    • pilot_weekly_active_users — Weekly active users across pilot orgs   │
  │      target: >5/org  unit: count  source: analytics  frequency: weekly   │
  └─────────────────────────────────────────────────────────────────────────┘

  Running red team review...  ████████████████ done (4.8s)

Step 3: Red Team Review — 7 panelists

  ✓ CEO (Jordan Park): APPROVES
    "These KPIs directly map to our Series A story. Enterprise conversion
     and velocity are exactly what the board wants to see."

  ✓ Head of Product (Priya Mehta): APPROVES
    "Enterprise Readiness with pilot feature tracking is right. The 80%
     threshold is correct — 100% would stall us chasing edge cases."

  ⚠ CFO (Marcus Webb): REQUESTS CHANGE
    "Revenue Conversion needs a cost metric alongside ARR pipeline.
     We could hit $400k pipeline and still burn through runway if CAC
     is unchecked. Add compliance spend vs. budget — we're at $40k of
     a $120k allocation and that's invisible right now."
    Suggested metric: compliance_remediation_spend
      unit: dollars  target: <$120k  source: manual  frequency: monthly

  ⚠ Tech Director (Elena Sousa): REQUESTS CHANGE
    "p95 alone misses tail latency. Last quarter we had a P0 where p99
     was 8 seconds while p95 looked fine. We also have no incident metric
     — enterprise pilots will churn on reliability before they churn on
     features."
    Suggested metric: p99_api_latency
      unit: ms  target: <800ms  source: datadog  frequency: daily
    Suggested metric: monthly_incident_count
      unit: count  target: <2  source: pagerduty  frequency: monthly

  ⚠ Lead Security Engineer (Sam Torres): REQUESTS CHANGE
    "SOC 2 control coverage is documentation. The real audit risk is mean
     time to remediate a CVE — if we find one in November and can't patch
     in 48h, the December audit fails regardless of what's documented."
    Suggested metric: mean_time_to_patch_cve
      unit: hours  target: <48h  source: manual  frequency: per_event
    Suggested metric: audit_log_integrity_checks
      unit: count  target: 100%  source: manual  frequency: weekly

  ✓ Head of Marketing (Alex Kim): APPROVES
    "Revenue Conversion and Enterprise Readiness tell the customer story
     I need for case studies. CSAT would be nice eventually but not now."

  ⚠ Head of Sales (Diana Ruiz): REQUESTS CHANGE
    "Time to close at <90 days doesn't account for enterprise procurement.
     That cycle is often 4-6 months and we don't control it. What matters
     is where deals stall in our process and whether pilots are engaged."
    Suggested metric: days_stalled_in_procurement
      unit: days  target: <30  source: manual  frequency: per_deal

  Consensus: NEEDS REVISION (3 Approve, 4 Request Change, 0 Reject)

Step 4: Resolve concerns

  Select concerns to address:
  [x] CFO: Add compliance_remediation_spend metric
  [x] Tech Director: Add p99_api_latency + incident count
  [x] Lead Security Engineer: Add CVE patch time + audit log integrity
  [x] Head of Sales: Replace time_to_close with days_stalled_in_procurement
  [ ] Tech Director: p99 tail latency — deferred to next quarter

  Response to CFO:
> Adding. Budget is $120k/year, we're at $40k now, and that's invisible to the
  team. Good catch.

  Response to Lead Security Engineer:
> CVE response time is critical before December. Sam is right that control
  documentation doesn't prove we can respond. Adding both metrics.

  Regenerating final KPIs...  ████████████████ done (2.1s)

Step 5: Preview final KPIs

  5 KPIs  •  17 metrics  •  2 high-weight (Enterprise Readiness 1.5, Revenue 1.3)

  All panel concerns resolved. Proceed to write meridian.toml? [Y/n] Y

  Written: /home/maya/stackwise/meridian.toml

  Next steps:
    meridian report           — score this week's sessions against these KPIs
    meridian pm refine        — revisit KPIs next quarter
    meridian report --since 30d --format csv | open -a Excel
```

---

## What was written

```toml
# meridian.toml — generated by meridian pm init

[source]
type = "claude-code"

[report]
format = "table"

[[kpi]]
id = "enterprise_readiness"
label = "Enterprise Readiness"
description = """
Ship the features enterprise pilots need to convert. Pilot-requested features,
SOC 2 control coverage, SSO and audit log availability, CVE response speed.
"""
weight = 1.5

  [[kpi.metrics]]
  id = "pilot_feature_completion"
  label = "Pilot feature completion"
  unit = "percent"
  target = ">80"
  source = "linear"
  frequency = "weekly"

  [[kpi.metrics]]
  id = "soc2_control_coverage"
  label = "SOC 2 control coverage"
  unit = "percent"
  target = "100"
  source = "manual"
  frequency = "weekly"

  [[kpi.metrics]]
  id = "mean_time_to_patch_cve"
  label = "Mean time to patch CVE"
  unit = "hours"
  target = "<48"
  source = "manual"
  frequency = "per_event"

  [[kpi.metrics]]
  id = "audit_log_integrity_checks"
  label = "Audit log integrity checks"
  unit = "count"
  target = "100%"
  source = "manual"
  frequency = "weekly"

[[kpi]]
id = "engineering_velocity"
label = "Engineering Velocity"
description = """
Keep shipping product features at pace despite compliance overhead.
PR merge time, review latency, and compliance session ratio.
"""
weight = 1.2

  [[kpi.metrics]]
  id = "mean_time_to_merge"
  label = "Mean time to merge"
  unit = "hours"
  target = "<24"
  source = "git"
  frequency = "weekly"

  [[kpi.metrics]]
  id = "compliance_overhead_ratio"
  label = "Compliance overhead ratio"
  unit = "percent"
  target = "<40"
  source = "ta_velocity"
  frequency = "weekly"

[[kpi]]
id = "dashboard_performance"
label = "Dashboard Performance"
description = "Close the performance gap vs. competitors. API latency, load time, incidents."
weight = 1.1

  [[kpi.metrics]]
  id = "p95_api_latency"
  label = "p95 API latency"
  unit = "ms"
  target = "<300"
  source = "datadog"
  frequency = "daily"

  [[kpi.metrics]]
  id = "p99_api_latency"
  label = "p99 API latency"
  unit = "ms"
  target = "<800"
  source = "datadog"
  frequency = "daily"

  [[kpi.metrics]]
  id = "monthly_incident_count"
  label = "Monthly incidents"
  unit = "count"
  target = "<2"
  source = "pagerduty"
  frequency = "monthly"

[[kpi]]
id = "revenue_conversion"
label = "Revenue Conversion"
description = "Convert pilots to paid annual contracts. Pipeline health and pilot engagement."
weight = 1.3

  [[kpi.metrics]]
  id = "pilot_to_paid_conversion"
  label = "Pilot to paid conversion"
  unit = "count"
  target = ">2/3"
  source = "manual"
  frequency = "monthly"

  [[kpi.metrics]]
  id = "arr_pipeline"
  label = "ARR pipeline"
  unit = "dollars"
  target = ">400000"
  source = "manual"
  frequency = "monthly"

  [[kpi.metrics]]
  id = "pilot_weekly_active_users"
  label = "Pilot weekly active users"
  unit = "count"
  target = ">5"
  source = "product_analytics"
  frequency = "weekly"

  [[kpi.metrics]]
  id = "days_stalled_in_procurement"
  label = "Days stalled in procurement"
  unit = "days"
  target = "<30"
  source = "manual"
  frequency = "per_deal"

[[kpi]]
id = "security_compliance"
label = "Security & Compliance"
description = "Security posture and compliance readiness for the December SOC 2 audit."
weight = 1.4

  [[kpi.metrics]]
  id = "compliance_remediation_spend"
  label = "Compliance remediation spend"
  unit = "dollars"
  target = "<120000"
  source = "manual"
  frequency = "monthly"
```

# `ops/` — Observability & alerting wiring for the swings admin platform

> **Last revised**: 2026-04-19

This directory ships **everything needed to stand the admin
platform's monitoring story up in a fresh environment**, without
hand-rolling alert expressions or guessing dashboard IDs:

```
ops/
├── prometheus/
│   └── admin-alerts.rules.yml      # Prometheus alerting rules
├── grafana/
│   └── admin-overview.dashboard.json  # Grafana dashboard (provisioning-ready)
└── README.md                       # this file
```

The runbook these files reference lives at
[`docs/RUNBOOK.md`](../docs/RUNBOOK.md) — every alert in
`admin-alerts.rules.yml` carries a `runbook_url` annotation pointing
at a section there. **Do not add a new alert without a corresponding
runbook entry.**

## Provisioning

### Prometheus rules

Drop `prometheus/admin-alerts.rules.yml` somewhere in your Prometheus
`rule_files:` glob, e.g.:

```yaml
# prometheus.yml
rule_files:
  - "/etc/prometheus/rules/*.rules.yml"
```

Validate before loading:

```bash
promtool check rules ops/prometheus/admin-alerts.rules.yml
```

### Grafana dashboard

The dashboard JSON is structured for Grafana's *file-based
provisioning* path. Mount it under
`/etc/grafana/provisioning/dashboards/` along with a provider config:

```yaml
# /etc/grafana/provisioning/dashboards/swings.yml
apiVersion: 1
providers:
  - name: 'swings'
    orgId: 1
    folder: 'Swings'
    type: file
    disableDeletion: false
    editable: true
    options:
      path: /etc/grafana/dashboards/swings
```

Then copy `grafana/admin-overview.dashboard.json` to
`/etc/grafana/dashboards/swings/`. The `${datasource}` template
variable will resolve to the default Prometheus datasource on first
load.

For ad-hoc imports: Grafana → **Dashboards → Import → Upload JSON**
and paste the file contents.

## Metric inventory

Every counter/gauge/histogram referenced in the rules + dashboards is
emitted by the Rust backend via the `metrics` facade. The full set as
of 2026-04-19:

| Metric | Type | Where emitted |
|---|---|---|
| `dsar_export_claimed_total` | counter | `services/dsar_worker.rs` |
| `dsar_export_completed_total` | counter | `services/dsar_worker.rs` |
| `dsar_export_failed_total{stage}` | counter | `services/dsar_worker.rs` |
| `dsar_export_duration_seconds` | histogram | `services/dsar_worker.rs` |
| `dsar_artifacts_swept_total` | counter | `services/dsar_artifact_sweep.rs` |
| `dsar_artifacts_sweep_duration_seconds` | histogram | `services/dsar_artifact_sweep.rs` |
| `dsar_artifacts_sweep_last_success_unixtime` | gauge | `services/dsar_artifact_sweep.rs` |
| `dsar_artifacts_sweep_failed_total{stage}` | counter | `services/dsar_artifact_sweep.rs` |
| `idempotency_claimed_total` | counter | `middleware/idempotency.rs` |
| `idempotency_replay_total` | counter | `middleware/idempotency.rs` |
| `idempotency_in_flight_total` | counter | `middleware/idempotency.rs` |
| `idempotency_mismatch_total` | counter | `middleware/idempotency.rs` |
| `idempotency_claim_race_total` | counter | `middleware/idempotency.rs` |
| `idempotency_db_error_total` | counter | `middleware/idempotency.rs` |
| `idempotency_keys_pruned_total` | counter | `services/idempotency_gc.rs` |
| `idempotency_keys_prune_duration_seconds` | histogram | `services/idempotency_gc.rs` |
| `idempotency_keys_prune_last_success_unixtime` | gauge | `services/idempotency_gc.rs` |
| `idempotency_keys_prune_failed_total` | counter | `services/idempotency_gc.rs` |
| `idempotency_keys_table_rows` | gauge | `services/idempotency_gc.rs` |
| `audit_pruned_total` | counter | `services/audit_retention.rs` |
| `audit_prune_iterations_total` | counter | `services/audit_retention.rs` |
| `audit_prune_duration_seconds` | histogram | `services/audit_retention.rs` |
| `audit_prune_errors_total` | counter | `services/audit_retention.rs` |
| `audit_prune_last_success_unixtime` | gauge | `services/audit_retention.rs` |
| `admin_mutation_rate_limited_total{backend}` | counter | `middleware/rate_limit.rs` |
| `impersonation_mint_rate_limited_total` | counter | `handlers/admin_impersonation.rs` |
| `impersonation_revoke_total` | counter | `security/impersonation.rs` |
| `maintenance_mode_blocked_total` | counter | `middleware/maintenance_mode.rs` |

The Rust side is configured to expose them via the existing Prometheus
exporter route (see `main.rs`); no scrape-config changes are required
beyond pointing Prometheus at the backend's `/metrics` endpoint.

## Adding a new alert

1. Add the rule to `prometheus/admin-alerts.rules.yml` under the
   correct `group:`. Keep the convention: severity label, team label,
   domain label, and a `runbook_url` annotation.
2. Add a section to `docs/RUNBOOK.md` with the same anchor as the
   `runbook_url`.
3. (Optional) Add a panel to `grafana/admin-overview.dashboard.json`
   so the on-call can eyeball the trend before the alert ever fires.
4. Run `promtool check rules ops/prometheus/admin-alerts.rules.yml`
   in CI; failing rules block merge.

## Adding a new metric

1. Emit from the Rust backend via the `metrics` facade.
2. Add a row to the inventory table above.
3. Decide whether it warrants an alert (most do not — gauges that
   already feed the dashboard often suffice).

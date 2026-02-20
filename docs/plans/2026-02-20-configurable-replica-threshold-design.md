# Configurable K8S002 Replica Threshold

**Date**: 2026-02-20
**Issue**: [#57 - K8S002: allow configuration for 2 replicas](https://github.com/clowdhaus/eksup/issues/57)
**Also addresses**: [#11 - K8S010: Migrate In-tree storage plugin to CSI driver](https://github.com/clowdhaus/eksup/issues/11) (close as no longer applicable)

## Problem

K8S002 hardcodes `< 3` replicas as a failure for all workloads (except a special-cased CoreDNS exception). For leader-election workloads (controllers, operators), 2 replicas is sufficient for HA. Users cannot override this threshold, leading to false positives.

## Solution

Introduce a config file (`.eksup.yaml`) with per-check configuration and per-workload overrides. Change the default minimum replica threshold from 3 to 2.

### Config File Format

```yaml
# .eksup.yaml
checks:
  K8S002:
    min_replicas: 2        # global default
    ignore:                 # skip these workloads entirely
      - name: singleton-worker
        namespace: batch
    overrides:              # per-workload thresholds
      - name: etcd-cluster
        namespace: infra
        min_replicas: 3
```

All fields are optional. Without a config file, behavior uses the new default of `min_replicas: 2`.

### Config Discovery

1. `--config <path>` CLI flag (explicit, highest priority)
2. `.eksup.yaml` in current working directory (implicit fallback)
3. No config found = use hardcoded defaults (min_replicas: 2)

### Key Design Decisions

- **Default lowered to 2**: The 3-replica requirement is only appropriate for quorum-based systems. 2 replicas is the practical HA minimum for rolling updates and leader-election.
- **CoreDNS special case removed**: With default at 2, the hardcoded `coredns` exception is unnecessary. The config system can handle any such cases generically.
- **Broad config structure**: The `checks:` top-level key supports future per-check config without restructuring. Only K8S002 is implemented now.
- **Ignore support**: Explicit ignore list for workloads that should be entirely excluded from K8S002 checking (e.g., intentionally single-replica batch processors).

## Changes

### New: `config.rs`

New module with:
- `Config` struct (top-level, with `checks: Option<ChecksConfig>`)
- `ChecksConfig` struct (with `k8s002: Option<K8s002Config>`)
- `K8s002Config` struct (with `min_replicas`, `ignore`, `overrides`)
- `ResourceSelector` struct (with `name`, `namespace`)
- `ReplicaOverride` struct (extends `ResourceSelector` with `min_replicas`)
- `Config::load(path: Option<&str>) -> Result<Config>` function
- `Config::default()` returning `min_replicas: 2`

### Modified: `lib.rs`

- Add `--config` flag to `Analysis` and `Playbook` CLI structs
- Load config via `Config::load()` before analysis
- Pass `&Config` to `analysis::analyze()`

### Modified: `analysis.rs`

- `analyze()` accepts `&Config` parameter
- Passes relevant config to `k8s::get_kubernetes_findings()`

### Modified: `k8s/findings.rs`

- `get_kubernetes_findings()` accepts `&K8s002Config`
- Passes config when collecting `min_replicas` findings

### Modified: `k8s/checks.rs`

- `K8sFindings::min_replicas()` trait method gains `&K8s002Config` parameter
- Success message updated: `"All relevant Kubernetes workloads meet the configured minimum replicas"`

### Modified: `k8s/resources.rs`

- `StdResource::min_replicas()` implementation updated:
  - Check ignore list first (match by name + namespace)
  - Check overrides for per-workload threshold
  - Fall back to global `min_replicas` from config
  - Remove hardcoded CoreDNS special case

### Modified: `finding.rs`

- Mark K8S010 as retired: `until: Some(24)` (same as K8S009)

### Tests

- Unit tests for config loading (valid YAML, missing file, invalid values)
- Unit tests for `min_replicas` with config (default, ignore, overrides)
- Update existing snapshot/integration tests for new default threshold

## Issue #11 (K8S010)

K8S010 ("EBS CSI driver not installed") was relevant for Kubernetes v1.23. The current supported range is v1.30-v1.35 — any cluster at this level has long completed the migration. Action: mark K8S010 as retired (`until: Some(24)`) and close issue #11.

## Backward Compatibility

The default `min_replicas` changes from 3 to 2. This is a deliberate behavior change — users who want the old threshold can set `min_replicas: 3` in their config file.

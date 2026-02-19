# Integration & E2E Tests Design

## Goal

Add integration and end-to-end tests using mocks to cover the currently untested async orchestration functions and verify user-facing output (text tables, JSON, playbook markdown).

## Current State

- 99 unit tests covering pure/synchronous check logic
- Zero integration or e2e tests
- No `[dev-dependencies]`, no mocking libraries
- All async functions that call AWS/K8s APIs are completely untested
- 5 check functions in `eks/checks.rs` mix resource fetching with check logic, making them unnecessarily hard to test

## Approach: Unified Trait Abstraction

Define traits over AWS and K8s clients. Production code uses real implementations; tests use mock implementations with pre-built data. Rust 2024 edition supports async fn in traits natively.

## Architectural Changes

### 1. Purify Async Check Functions

Pull resource fetching out of 5 async check functions in `eks/checks.rs` up into `eks/findings.rs`. The checks become pure functions that receive pre-fetched data:

| Function | Currently fetches | After refactor |
|---|---|---|
| `control_plane_ips` | `get_subnet_ips()` | Takes `&[VpcSubnet]`, cluster subnet config |
| `pod_ips` | `get_eniconfigs()` + `get_subnet_ips()` | Takes `&[VpcSubnet]`, thresholds |
| `addon_version_compatibility` | `get_addon_versions()` per addon | Takes addons + pre-fetched `AddonVersion` maps |
| `eks_managed_nodegroup_update` | `get_launch_template()` | Takes nodegroup + `LaunchTemplate` |
| `self_managed_nodegroup_update` | `get_launch_template()` | Takes ASG + `LaunchTemplate` |

This makes them unit-testable without mocking (same pattern as `cluster_health`, `addon_health`, etc.).

### 2. Trait Definitions

New module: `src/clients.rs`

**`AwsClients` trait** (8 methods):
- `get_cluster(name) -> Result<Cluster>`
- `get_subnet_ips(subnet_ids) -> Result<Vec<VpcSubnet>>`
- `get_addons(cluster_name) -> Result<Vec<Addon>>`
- `get_addon_versions(name, k8s_version) -> Result<AddonVersion>`
- `get_eks_managed_nodegroups(cluster_name) -> Result<Vec<Nodegroup>>`
- `get_self_managed_nodegroups(cluster_name) -> Result<Vec<AutoScalingGroup>>`
- `get_fargate_profiles(cluster_name) -> Result<Vec<FargateProfile>>`
- `get_launch_template(id) -> Result<LaunchTemplate>`

**`K8sClients` trait** (4 methods):
- `get_nodes() -> Result<Vec<Node>>`
- `get_configmap(namespace, name) -> Result<Option<ConfigMap>>`
- `get_eniconfigs() -> Result<Vec<ENIConfig>>`
- `get_resources() -> Result<Vec<StdResource>>`

**`RealAwsClients`** struct: holds `EksClient`, `Ec2Client`, `AsgClient`. Implements `AwsClients` using existing resource functions as method bodies.

**`RealK8sClients`** struct: holds `kube::Client`. Implements `K8sClients` using existing resource functions as method bodies.

### 3. Production Code Refactoring

Three layers change (mechanical — swap parameter types):

**Layer 1: `eks/resources.rs` and `k8s/resources.rs`**
- Trait definitions and real implementations live here (or in `clients.rs`)
- Existing free functions become method bodies of real impls

**Layer 2: `eks/findings.rs`, `k8s/findings.rs`**
- Orchestration functions take `&(impl AwsClients)` / `&(impl K8sClients)` instead of concrete SDK clients
- `eks/findings.rs` gains the resource-fetching that was pulled out of checks

**Layer 3: `analysis.rs`**
- `analyze()` takes `&(impl AwsClients)` + `&(impl K8sClients)` instead of `&SdkConfig`
- Uses `tokio::try_join!` to run independent findings concurrently

**Layer 4: `lib.rs`**
- Constructs `RealAwsClients` and `RealK8sClients` from `SdkConfig`
- Only place that knows about concrete SDK client types

### 4. Concurrent Findings Collection

`analysis::analyze` uses `tokio::try_join!` for independent findings:
- `get_cluster_findings` (sync, but cheap)
- `get_subnet_findings` (AWS EC2 + K8s)
- `get_addon_findings` (AWS EKS)
- `get_data_plane_findings` (AWS ASG + EC2 + EKS)
- `get_kubernetes_findings` (K8s)

## Test Architecture

### Dependencies

```toml
[dev-dependencies]
insta = { version = "1", features = ["yaml"] }
```

`tokio` is already in `[dependencies]` with the needed features.

### Mock Design

Simple structs with pre-built return data. No mocking framework.

```rust
struct MockAwsClients {
    cluster: Cluster,
    subnet_ips: Vec<VpcSubnet>,
    addons: Vec<Addon>,
    addon_versions: HashMap<(String, String), AddonVersion>,
    nodegroups: Vec<Nodegroup>,
    self_managed_nodegroups: Vec<AutoScalingGroup>,
    fargate_profiles: Vec<FargateProfile>,
    launch_templates: HashMap<String, LaunchTemplate>,
}
```

`Default` impl provides a "healthy cluster" baseline. Individual tests override specific fields.

Error path testing uses a separate `MockAwsClientsError` struct that returns `Err(...)` for specific methods.

### Test File Structure

```
eksup/tests/
  common/
    mod.rs            # re-exports
    mock_aws.rs       # MockAwsClients + Default + builder helpers
    mock_k8s.rs       # MockK8sClients + Default + builder helpers
    fixtures.rs       # Reusable test data (clusters, nodegroups, addons, etc.)
  integration/
    mod.rs
    eks_findings.rs   # get_cluster/subnet/addon/data_plane_findings
    k8s_findings.rs   # get_kubernetes_findings
    analysis.rs       # Full analysis::analyze pipeline
  e2e/
    mod.rs
    text_output.rs    # Snapshot: text table output
    json_output.rs    # Snapshot: JSON output
    playbook.rs       # Snapshot: playbook markdown
  snapshots/          # Auto-managed by insta
```

### Test Scenarios

| Scenario | Findings exercised | Output formats |
|---|---|---|
| `healthy_cluster` | None | text, JSON |
| `insufficient_control_plane_ips` | AWS001 | text, JSON |
| `insufficient_pod_ips` | AWS002 | text, JSON |
| `cluster_health_issues` | EKS002 | text, JSON |
| `nodegroup_health_issues` | EKS003 | text, JSON |
| `addon_health_issues` | EKS004 | text, JSON |
| `addon_version_incompatible` | EKS005 | text, JSON |
| `launch_template_drift_managed` | EKS006 | text, JSON |
| `launch_template_drift_self_managed` | EKS007 | text, JSON |
| `al2_ami_deprecation` | EKS008 | text, JSON |
| `node_version_skew` | K8S001 | text, JSON |
| `workload_best_practices` | K8S002-K8S008 | text, JSON |
| `kube_proxy_issues` | K8S009, K8S010 | text, JSON |
| `ingress_nginx_retirement` | K8S011 | text, JSON |
| `mixed_findings` | Multiple codes | text, JSON, playbook |
| `ignore_recommended` | Filtered output | text, JSON |
| `error_aws_api_failure` | Error propagation | N/A |
| `error_k8s_api_failure` | Error propagation | N/A |

### New Unit Tests (from purified checks)

~20-30 additional unit tests for the 5 purified check functions, covering edge cases:
- `control_plane_ips`: 0 AZs, 1 AZ with enough, 2 AZs with enough, boundary at exactly 5
- `pod_ips`: no ENIConfigs, below required, between required and recommended, above recommended
- `addon_version_compatibility`: supported current, unsupported on target, not latest (recommended)
- `eks_managed_nodegroup_update`: no launch template, current == latest, current != latest
- `self_managed_nodegroup_update`: same cases

## Expected Coverage Gains

| Category | Before | After |
|---|---|---|
| Pure check functions | Tested (99 tests) | Tested (~125 tests, +26 from purified checks) |
| Orchestration (findings.rs) | Untested | Tested (~15-20 integration tests) |
| Output formatting | 3 tests | Snapshot tested (~10-15 e2e tests) |
| Playbook generation | Untested | Snapshot tested |
| Error paths | Untested | Tested (~5 error path tests) |

## Principles

- No logic changes to existing checks — refactoring is mechanical
- Existing 99 unit tests continue to pass unchanged
- Mocks are plain Rust structs — no framework dependency
- Snapshots make output changes visible in PR reviews
- Tests are fast — no network, no filesystem (except snapshot files)

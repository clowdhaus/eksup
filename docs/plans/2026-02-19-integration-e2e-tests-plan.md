# Integration & E2E Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add trait-based mocking, purify async checks, and implement integration + e2e snapshot tests covering all previously untested orchestration and output code.

**Architecture:** Define `AwsClients` and `K8sClients` traits over the AWS SDK and kube-rs clients. Refactor 5 async check functions to be pure (fetching moves to findings.rs). Mock implementations enable integration tests of orchestration functions. `insta` snapshots verify user-facing text, JSON, and playbook output.

**Tech Stack:** Rust 2024 edition, `insta` (snapshot testing), `tokio` (async runtime), AWS SDK for Rust, kube-rs

---

### Task 1: Add dev-dependencies

**Files:**
- Modify: `eksup/Cargo.toml`

**Step 1: Add insta to dev-dependencies**

Add after the `[dependencies]` section and before `[lints.rust]`:

```toml
[dev-dependencies]
insta = { version = "1", features = ["yaml"] }
```

**Step 2: Verify it compiles**

Run: `cargo check -p eksup`
Expected: compiles with no errors

**Step 3: Commit**

```bash
git add eksup/Cargo.toml
git commit -m "build: add insta dev-dependency for snapshot testing"
```

---

### Task 2: Purify async check functions

Refactor 5 async check functions in `eks/checks.rs` to be pure (sync) by removing their internal resource fetching. Move the fetching into `eks/findings.rs`.

**Files:**
- Modify: `eksup/src/eks/checks.rs` (lines 101-190, 221-261, 373-464)
- Modify: `eksup/src/eks/findings.rs` (lines 43-58, 75-90, 118-170)
- Modify: `eksup/src/eks/mod.rs`
- Modify: `eksup/src/k8s/mod.rs`

**Step 1: Purify `control_plane_ips` in `eks/checks.rs`**

Change from async function taking `&Ec2Client` to pure function taking pre-fetched data.

Before (lines 101-137):
```rust
pub(crate) async fn control_plane_ips(ec2_client: &Ec2Client, cluster: &Cluster) -> Result<Vec<InsufficientSubnetIps>> {
  let subnet_ids = match cluster.resources_vpc_config() {
    Some(vpc_config) => vpc_config.subnet_ids().to_owned(),
    None => return Ok(vec![]),
  };

  let subnet_ips = resources::get_subnet_ips(ec2_client, subnet_ids).await?;
  // ... rest of check logic
```

After:
```rust
pub(crate) fn control_plane_ips(subnet_ips: &[resources::VpcSubnet]) -> Vec<InsufficientSubnetIps> {
  let mut az_ips: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
  for subnet in subnet_ips {
    *az_ips.entry(subnet.availability_zone_id.clone()).or_default() += subnet.available_ips;
  }
  let availability_zone_ips: Vec<(String, i32)> = az_ips.into_iter().collect();

  if availability_zone_ips
    .iter()
    .filter(|(_az, ips)| ips >= &5)
    .count()
    >= 2
  {
    return vec![];
  }

  let finding = Finding::new(Code::EKS001, Remediation::Required);

  availability_zone_ips
    .iter()
    .map(|(az, ips)| InsufficientSubnetIps {
      finding: finding.clone(),
      id: az.clone(),
      available_ips: *ips,
    })
    .collect()
}
```

**Step 2: Purify `pod_ips` in `eks/checks.rs`**

Before (lines 144-190):
```rust
pub(crate) async fn pod_ips(
  ec2_client: &Ec2Client,
  k8s_client: &K8sClient,
  required_ips: i32,
  recommended_ips: i32,
) -> Result<Vec<InsufficientSubnetIps>> {
  let eniconfigs = k8s::get_eniconfigs(k8s_client).await?;
  if eniconfigs.is_empty() { return Ok(vec![]); }
  let subnet_ids = eniconfigs.iter().filter_map(|e| e.spec.subnet.clone()).collect();
  let subnet_ips = resources::get_subnet_ips(ec2_client, subnet_ids).await?;
  // ... check logic
```

After:
```rust
pub(crate) fn pod_ips(
  subnet_ips: &[resources::VpcSubnet],
  required_ips: i32,
  recommended_ips: i32,
) -> Vec<InsufficientSubnetIps> {
  if subnet_ips.is_empty() {
    return vec![];
  }

  let available_ips: i32 = subnet_ips.iter().map(|subnet| subnet.available_ips).sum();

  if available_ips >= recommended_ips {
    return vec![];
  }

  let remediation = if available_ips < required_ips {
    Remediation::Required
  } else {
    Remediation::Recommended
  };

  let finding = Finding::new(Code::AWS002, remediation);

  let mut az_ips: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
  for subnet in subnet_ips {
    *az_ips.entry(subnet.availability_zone_id.clone()).or_default() += subnet.available_ips;
  }

  az_ips
    .into_iter()
    .map(|(az, ips)| InsufficientSubnetIps {
      finding: finding.clone(),
      id: az,
      available_ips: ips,
    })
    .collect()
}
```

**Step 3: Purify `addon_version_compatibility` in `eks/checks.rs`**

Before (lines 221-261):
```rust
pub(crate) async fn addon_version_compatibility(
  client: &EksClient,
  cluster_version: &str,
  target_minor: i32,
  addons: &[Addon],
) -> Result<Vec<AddonVersionCompatibility>> {
```

After — takes pre-fetched version data instead of the EKS client:
```rust
pub(crate) fn addon_version_compatibility(
  addons: &[Addon],
  current_versions: &HashMap<String, resources::AddonVersion>,
  target_versions: &HashMap<String, resources::AddonVersion>,
) -> Vec<AddonVersionCompatibility> {
  let mut addon_findings = Vec::new();

  for addon in addons {
    let name = addon.addon_name().unwrap_or_default().to_owned();
    let version = addon.addon_version().unwrap_or_default().to_owned();

    let current_kubernetes_version = match current_versions.get(&name) {
      Some(v) => v.clone(),
      None => continue,
    };
    let target_kubernetes_version = match target_versions.get(&name) {
      Some(v) => v.clone(),
      None => continue,
    };

    let remediation = if !target_kubernetes_version.supported_versions.contains(&version)
      || !current_kubernetes_version.supported_versions.contains(&version)
    {
      Some(Remediation::Required)
    } else if current_kubernetes_version.latest != version {
      Some(Remediation::Recommended)
    } else {
      None
    };

    if let Some(remediation) = remediation {
      addon_findings.push(AddonVersionCompatibility {
        finding: Finding::new(Code::EKS005, remediation),
        name,
        version,
        current_kubernetes_version,
        target_kubernetes_version,
      })
    }
  }

  addon_findings
}
```

Add `use std::collections::HashMap;` to the imports at the top of `eks/checks.rs`.

**Step 4: Purify `eks_managed_nodegroup_update` in `eks/checks.rs`**

Before (lines 373-415):
```rust
pub(crate) async fn eks_managed_nodegroup_update(
  client: &Ec2Client,
  nodegroup: &Nodegroup,
) -> Result<Vec<ManagedNodeGroupUpdate>> {
```

After — takes an optional pre-fetched launch template:
```rust
pub(crate) fn eks_managed_nodegroup_update(
  nodegroup: &Nodegroup,
  launch_template: Option<&resources::LaunchTemplate>,
) -> Vec<ManagedNodeGroupUpdate> {
  let launch_template = match launch_template {
    Some(lt) => lt,
    None => return vec![],
  };

  match nodegroup.resources() {
    Some(resources) => {
      resources
        .auto_scaling_groups()
        .iter()
        .map(|asg| {
          ManagedNodeGroupUpdate {
            finding: Finding::new(Code::EKS006, Remediation::Recommended),
            name: nodegroup.nodegroup_name().unwrap_or_default().to_owned(),
            autoscaling_group_name: asg.name().unwrap_or_default().to_owned(),
            launch_template: launch_template.to_owned(),
          }
        })
        .filter(|asg| asg.launch_template.current_version != asg.launch_template.latest_version)
        .collect()
    }
    None => vec![],
  }
}
```

**Step 5: Purify `self_managed_nodegroup_update` in `eks/checks.rs`**

Before (lines 442-464):
```rust
pub(crate) async fn self_managed_nodegroup_update(
  client: &Ec2Client,
  asg: &AutoScalingGroup,
) -> Result<Option<AutoscalingGroupUpdate>> {
```

After — takes a pre-fetched launch template:
```rust
pub(crate) fn self_managed_nodegroup_update(
  asg: &AutoScalingGroup,
  launch_template: &resources::LaunchTemplate,
) -> Option<AutoscalingGroupUpdate> {
  let name = asg.auto_scaling_group_name().unwrap_or_default().to_owned();

  if launch_template.current_version != launch_template.latest_version {
    Some(AutoscalingGroupUpdate {
      finding: Finding::new(Code::EKS007, Remediation::Recommended),
      name,
      launch_template: launch_template.to_owned(),
    })
  } else {
    None
  }
}
```

**Step 6: Remove unused imports from `eks/checks.rs`**

After purification, these imports are no longer needed in `eks/checks.rs`:
- `use aws_sdk_ec2::Client as Ec2Client;`
- `use aws_sdk_eks::Client as EksClient;`
- `use kube::Client as K8sClient;`
- `use crate::eks::resources;` — keep this, still used for types
- `use crate::k8s;` — remove
- `use crate::version;` — remove (was used in addon_version_compatibility)

The remaining imports should be:
```rust
use std::collections::HashMap;

use anyhow::{Context, Result};
use aws_sdk_autoscaling::types::AutoScalingGroup;
use aws_sdk_eks::types::{Addon, AmiTypes, Cluster, Nodegroup};
use serde::{Deserialize, Serialize};
use tabled::{
  Table, Tabled,
  settings::{Margin, Style},
};

use crate::{
  eks::resources,
  finding::{self, Code, Finding, Findings, Remediation},
  output::tabled_vec_to_string,
};
```

**Step 7: Update `get_subnet_findings` in `eks/findings.rs`**

The fetching that was removed from checks moves here. Replace the current function (lines 43-58):

```rust
pub async fn get_subnet_findings(
  ec2_client: &Ec2Client,
  k8s_client: &K8sClient,
  cluster: &Cluster,
) -> Result<SubnetFindings> {
  // Fetch control plane subnet IPs
  let control_plane_subnet_ids = match cluster.resources_vpc_config() {
    Some(vpc_config) => vpc_config.subnet_ids().to_owned(),
    None => vec![],
  };
  let control_plane_subnet_ips = if control_plane_subnet_ids.is_empty() {
    vec![]
  } else {
    resources::get_subnet_ips(ec2_client, control_plane_subnet_ids).await?
  };

  // Fetch pod subnet IPs (custom networking via ENIConfig)
  let eniconfigs = crate::k8s::get_eniconfigs(k8s_client).await?;
  let pod_subnet_ids: Vec<String> = eniconfigs
    .iter()
    .filter_map(|eniconfig| eniconfig.spec.subnet.clone())
    .collect();
  let pod_subnet_ips = if pod_subnet_ids.is_empty() {
    vec![]
  } else {
    resources::get_subnet_ips(ec2_client, pod_subnet_ids).await?
  };

  // Run pure checks
  let control_plane_ips = checks::control_plane_ips(&control_plane_subnet_ips);
  let pod_ips = checks::pod_ips(&pod_subnet_ips, 16, 256);

  Ok(SubnetFindings {
    control_plane_ips,
    pod_ips,
  })
}
```

**Step 8: Update `get_addon_findings` in `eks/findings.rs`**

Replace the current function (lines 75-90):

```rust
pub async fn get_addon_findings(
  eks_client: &EksClient,
  cluster_name: &str,
  cluster_version: &str,
  target_minor: i32,
) -> Result<AddonFindings> {
  let addons = resources::get_addons(eks_client, cluster_name).await?;
  let target_k8s_version = crate::version::format_version(target_minor);

  // Pre-fetch all addon version data
  let mut current_versions = std::collections::HashMap::new();
  let mut target_versions = std::collections::HashMap::new();
  for addon in &addons {
    let name = addon.addon_name().unwrap_or_default().to_owned();
    let current = resources::get_addon_versions(eks_client, &name, cluster_version).await?;
    let target = resources::get_addon_versions(eks_client, &name, &target_k8s_version).await?;
    current_versions.insert(name.clone(), current);
    target_versions.insert(name, target);
  }

  let version_compatibility = checks::addon_version_compatibility(&addons, &current_versions, &target_versions);
  let health = checks::addon_health(&addons)?;

  Ok(AddonFindings {
    version_compatibility,
    health,
  })
}
```

**Step 9: Update `get_data_plane_findings` in `eks/findings.rs`**

Replace the current function (lines 118-170):

```rust
pub async fn get_data_plane_findings(
  asg_client: &AsgClient,
  ec2_client: &Ec2Client,
  eks_client: &EksClient,
  cluster: &Cluster,
  target_minor: i32,
) -> Result<DataPlaneFindings> {
  let cluster_name = cluster.name().unwrap_or_default();

  let eks_mngs = resources::get_eks_managed_nodegroups(eks_client, cluster_name).await?;
  let self_mngs = resources::get_self_managed_nodegroups(asg_client, cluster_name).await?;
  let fargate_profiles = resources::get_fargate_profiles(eks_client, cluster_name).await?;

  let eks_managed_nodegroup_health = checks::eks_managed_nodegroup_health(&eks_mngs)?;
  let al2_ami_deprecation = checks::al2_ami_deprecation(&eks_mngs, target_minor)?;

  // Pre-fetch launch templates for EKS managed nodegroups, then run pure check
  let mut eks_managed_nodegroup_update = Vec::new();
  for eks_mng in &eks_mngs {
    let lt = match eks_mng.launch_template() {
      Some(lt_spec) => {
        let lt_id = lt_spec.id().context("Launch template spec missing ID")?;
        Some(resources::get_launch_template(ec2_client, lt_id).await?)
      }
      None => None,
    };
    eks_managed_nodegroup_update.extend(checks::eks_managed_nodegroup_update(eks_mng, lt.as_ref()));
  }

  // Pre-fetch launch templates for self-managed nodegroups, then run pure check
  let mut self_managed_nodegroup_update = Vec::new();
  for self_mng in &self_mngs {
    let lt_spec = self_mng
      .launch_template()
      .context("Launch template not found, launch configuration is not supported")?;
    let lt = resources::get_launch_template(ec2_client, lt_spec.launch_template_id().unwrap_or_default()).await?;
    if let Some(update) = checks::self_managed_nodegroup_update(self_mng, &lt) {
      self_managed_nodegroup_update.push(update);
    }
  }

  Ok(DataPlaneFindings {
    eks_managed_nodegroup_health,
    eks_managed_nodegroup_update,
    self_managed_nodegroup_update,
    al2_ami_deprecation,
    eks_managed_nodegroups: eks_mngs
      .iter()
      .map(|mng| mng.nodegroup_name().unwrap_or_default().to_owned())
      .collect(),
    self_managed_nodegroups: self_mngs
      .iter()
      .map(|asg| asg.auto_scaling_group_name().unwrap_or_default().to_owned())
      .collect(),
    fargate_profiles: fargate_profiles
      .iter()
      .map(|fp| fp.fargate_profile_name().unwrap_or_default().to_owned())
      .collect(),
  })
}
```

**Step 10: Make `get_cluster_findings` sync in `eks/findings.rs`**

It doesn't await anything, so drop the `async`:

```rust
pub fn get_cluster_findings(cluster: &Cluster) -> Result<ClusterFindings> {
  let cluster_health = checks::cluster_health(cluster)?;
  Ok(ClusterFindings { cluster_health })
}
```

Update `eks/mod.rs` re-export if needed (the function name stays the same).

Update `analysis.rs` caller from `eks::get_cluster_findings(cluster).await?` to `eks::get_cluster_findings(cluster)?`.

**Step 11: Verify all existing tests pass**

Run: `cargo test -p eksup`
Expected: All 99 existing tests pass. No logic changed, only function signatures.

**Step 12: Commit**

```bash
git add eksup/src/eks/checks.rs eksup/src/eks/findings.rs eksup/src/eks/mod.rs eksup/src/analysis.rs
git commit -m "refactor: purify 5 async check functions into sync pure functions

Move resource fetching from eks/checks.rs into eks/findings.rs so that
check functions receive pre-fetched data. This makes them unit-testable
without mocking. Also make get_cluster_findings sync."
```

---

### Task 3: Add unit tests for purified check functions

Add tests for the 5 newly-pure functions. These follow the same pattern as existing tests — construct data with SDK builders, call the pure function, assert on results.

**Files:**
- Modify: `eksup/src/eks/checks.rs` (add to existing `#[cfg(test)] mod tests` block)

**Step 1: Add `control_plane_ips` tests**

Add inside the existing `mod tests` block in `eks/checks.rs`:

```rust
  use crate::eks::resources::VpcSubnet;

  // ---------- control_plane_ips ----------

  #[test]
  fn control_plane_ips_empty_subnets() {
    let result = control_plane_ips(&[]);
    assert!(result.is_empty());
  }

  #[test]
  fn control_plane_ips_two_azs_sufficient() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 10, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 8, availability_zone_id: "use1-az2".into() },
    ];
    let result = control_plane_ips(&subnets);
    assert!(result.is_empty(), "2 AZs with >= 5 IPs should produce no findings");
  }

  #[test]
  fn control_plane_ips_one_az_insufficient() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 10, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 3, availability_zone_id: "use1-az2".into() },
    ];
    let result = control_plane_ips(&subnets);
    assert!(!result.is_empty(), "only 1 AZ with >= 5 IPs should produce findings");
    assert!(result.iter().all(|f| matches!(f.finding.remediation, Remediation::Required)));
  }

  #[test]
  fn control_plane_ips_boundary_exactly_5() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 5, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 5, availability_zone_id: "use1-az2".into() },
    ];
    let result = control_plane_ips(&subnets);
    assert!(result.is_empty(), "exactly 5 IPs in 2 AZs should pass");
  }

  #[test]
  fn control_plane_ips_aggregates_across_subnets_in_same_az() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1a".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-1b".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 6, availability_zone_id: "use1-az2".into() },
    ];
    let result = control_plane_ips(&subnets);
    assert!(result.is_empty(), "3+3=6 in az1 and 6 in az2 should pass");
  }
```

**Step 2: Add `pod_ips` tests**

```rust
  // ---------- pod_ips ----------

  #[test]
  fn pod_ips_empty_subnets() {
    let result = pod_ips(&[], 16, 256);
    assert!(result.is_empty(), "no subnets means no custom networking, no findings");
  }

  #[test]
  fn pod_ips_above_recommended() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 200, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 100, availability_zone_id: "use1-az2".into() },
    ];
    let result = pod_ips(&subnets, 16, 256);
    assert!(result.is_empty(), "300 IPs >= 256 recommended threshold");
  }

  #[test]
  fn pod_ips_between_required_and_recommended() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 100, availability_zone_id: "use1-az1".into() },
    ];
    let result = pod_ips(&subnets, 16, 256);
    assert!(!result.is_empty());
    assert!(result.iter().all(|f| matches!(f.finding.remediation, Remediation::Recommended)));
  }

  #[test]
  fn pod_ips_below_required() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 10, availability_zone_id: "use1-az1".into() },
    ];
    let result = pod_ips(&subnets, 16, 256);
    assert!(!result.is_empty());
    assert!(result.iter().all(|f| matches!(f.finding.remediation, Remediation::Required)));
  }
```

**Step 3: Add `addon_version_compatibility` tests**

```rust
  use std::collections::{HashMap, HashSet};
  use crate::eks::resources::AddonVersion;

  // ---------- addon_version_compatibility ----------

  #[test]
  fn addon_version_compat_all_supported() {
    let addon = Addon::builder()
      .addon_name("vpc-cni")
      .addon_version("v1.15.0")
      .build();

    let current = HashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.15.0".into(),
      default: "v1.14.0".into(),
      supported_versions: HashSet::from(["v1.15.0".into(), "v1.14.0".into()]),
    })]);
    let target = HashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.16.0".into(),
      default: "v1.15.0".into(),
      supported_versions: HashSet::from(["v1.16.0".into(), "v1.15.0".into()]),
    })]);

    let result = addon_version_compatibility(&[addon], &current, &target);
    assert!(result.is_empty(), "version supported in both should produce no findings");
  }

  #[test]
  fn addon_version_compat_not_latest_recommended() {
    let addon = Addon::builder()
      .addon_name("vpc-cni")
      .addon_version("v1.14.0")
      .build();

    let current = HashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.15.0".into(),
      default: "v1.14.0".into(),
      supported_versions: HashSet::from(["v1.15.0".into(), "v1.14.0".into()]),
    })]);
    let target = HashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.16.0".into(),
      default: "v1.15.0".into(),
      supported_versions: HashSet::from(["v1.16.0".into(), "v1.15.0".into(), "v1.14.0".into()]),
    })]);

    let result = addon_version_compatibility(&[addon], &current, &target);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
  }

  #[test]
  fn addon_version_compat_unsupported_on_target_required() {
    let addon = Addon::builder()
      .addon_name("vpc-cni")
      .addon_version("v1.12.0")
      .build();

    let current = HashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.15.0".into(),
      default: "v1.14.0".into(),
      supported_versions: HashSet::from(["v1.15.0".into(), "v1.14.0".into(), "v1.12.0".into()]),
    })]);
    let target = HashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.16.0".into(),
      default: "v1.15.0".into(),
      supported_versions: HashSet::from(["v1.16.0".into(), "v1.15.0".into()]),
    })]);

    let result = addon_version_compatibility(&[addon], &current, &target);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Required));
  }
```

**Step 4: Add `eks_managed_nodegroup_update` and `self_managed_nodegroup_update` tests**

```rust
  use aws_sdk_eks::types::{AutoScalingGroupProvider, NodegroupResources};
  use crate::eks::resources::LaunchTemplate;

  // ---------- eks_managed_nodegroup_update ----------

  #[test]
  fn mng_update_no_launch_template() {
    let ng = Nodegroup::builder().nodegroup_name("test").build();
    let result = eks_managed_nodegroup_update(&ng, None);
    assert!(result.is_empty());
  }

  #[test]
  fn mng_update_current_equals_latest() {
    let ng = Nodegroup::builder()
      .nodegroup_name("test")
      .resources(
        NodegroupResources::builder()
          .auto_scaling_groups(
            AutoScalingGroupProvider::builder().name("asg-1").build()
          )
          .build()
      )
      .build();
    let lt = LaunchTemplate {
      name: "lt-1".into(),
      id: "lt-abc".into(),
      current_version: "3".into(),
      latest_version: "3".into(),
    };
    let result = eks_managed_nodegroup_update(&ng, Some(&lt));
    assert!(result.is_empty(), "current == latest should produce no findings");
  }

  #[test]
  fn mng_update_current_behind_latest() {
    let ng = Nodegroup::builder()
      .nodegroup_name("test")
      .resources(
        NodegroupResources::builder()
          .auto_scaling_groups(
            AutoScalingGroupProvider::builder().name("asg-1").build()
          )
          .build()
      )
      .build();
    let lt = LaunchTemplate {
      name: "lt-1".into(),
      id: "lt-abc".into(),
      current_version: "2".into(),
      latest_version: "5".into(),
    };
    let result = eks_managed_nodegroup_update(&ng, Some(&lt));
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
  }

  // ---------- self_managed_nodegroup_update ----------

  #[test]
  fn smng_update_current_equals_latest() {
    let asg = AutoScalingGroup::builder()
      .auto_scaling_group_name("asg-1")
      .build();
    let lt = LaunchTemplate {
      name: "lt-1".into(),
      id: "lt-abc".into(),
      current_version: "3".into(),
      latest_version: "3".into(),
    };
    let result = self_managed_nodegroup_update(&asg, &lt);
    assert!(result.is_none());
  }

  #[test]
  fn smng_update_current_behind_latest() {
    let asg = AutoScalingGroup::builder()
      .auto_scaling_group_name("asg-1")
      .build();
    let lt = LaunchTemplate {
      name: "lt-1".into(),
      id: "lt-abc".into(),
      current_version: "1".into(),
      latest_version: "3".into(),
    };
    let result = self_managed_nodegroup_update(&asg, &lt);
    assert!(result.is_some());
    assert!(matches!(result.unwrap().finding.remediation, Remediation::Recommended));
  }
```

**Step 5: Run tests**

Run: `cargo test -p eksup`
Expected: All existing 99 tests pass + ~15 new tests pass

**Step 6: Commit**

```bash
git add eksup/src/eks/checks.rs
git commit -m "test: add unit tests for purified check functions"
```

---

### Task 4: Define client traits and real implementations

Create trait abstractions over AWS and K8s clients. Real implementations delegate to existing resource functions.

**Files:**
- Create: `eksup/src/clients.rs`
- Modify: `eksup/src/lib.rs` (add `pub mod clients;`)
- Modify: `eksup/src/eks/mod.rs` (make `resources` pub(crate))
- Modify: `eksup/src/k8s/mod.rs` (make `resources` pub(crate))
- Modify: `eksup/src/eks/resources.rs` (make `VpcSubnet` pub)

**Step 1: Make resource modules accessible**

In `eksup/src/eks/mod.rs`, change:
```rust
mod resources;
```
to:
```rust
pub(crate) mod resources;
```

In `eksup/src/k8s/mod.rs`, change:
```rust
mod resources;
```
to:
```rust
pub(crate) mod resources;
```

In `eksup/src/eks/resources.rs`, change `VpcSubnet` visibility:
```rust
pub struct VpcSubnet {
```
(remove `(crate)`)

Also change `get_subnet_ips`, `get_addon_versions`, and `get_launch_template` from `pub(crate)` to `pub`:
```rust
pub async fn get_subnet_ips(...
pub async fn get_addon_versions(...
pub async fn get_launch_template(...
```

**Step 2: Create `eksup/src/clients.rs`**

```rust
use anyhow::Result;
use aws_sdk_autoscaling::types::AutoScalingGroup;
use aws_sdk_eks::types::{Addon, Cluster, FargateProfile, Nodegroup};
use k8s_openapi::api::core::v1::ConfigMap;

use crate::{
  eks::resources::{self as eks_resources, AddonVersion, LaunchTemplate, VpcSubnet},
  k8s::resources::{self as k8s_resources, ENIConfig, Node, StdResource},
};

/// Trait abstracting all AWS API operations used by eksup
pub trait AwsClients {
  fn get_cluster(&self, name: &str) -> impl std::future::Future<Output = Result<Cluster>> + Send;
  fn get_subnet_ips(&self, subnet_ids: Vec<String>) -> impl std::future::Future<Output = Result<Vec<VpcSubnet>>> + Send;
  fn get_addons(&self, cluster_name: &str) -> impl std::future::Future<Output = Result<Vec<Addon>>> + Send;
  fn get_addon_versions(&self, name: &str, kubernetes_version: &str) -> impl std::future::Future<Output = Result<AddonVersion>> + Send;
  fn get_eks_managed_nodegroups(&self, cluster_name: &str) -> impl std::future::Future<Output = Result<Vec<Nodegroup>>> + Send;
  fn get_self_managed_nodegroups(&self, cluster_name: &str) -> impl std::future::Future<Output = Result<Vec<AutoScalingGroup>>> + Send;
  fn get_fargate_profiles(&self, cluster_name: &str) -> impl std::future::Future<Output = Result<Vec<FargateProfile>>> + Send;
  fn get_launch_template(&self, id: &str) -> impl std::future::Future<Output = Result<LaunchTemplate>> + Send;
}

/// Trait abstracting all Kubernetes API operations used by eksup
pub trait K8sClients {
  fn get_nodes(&self) -> impl std::future::Future<Output = Result<Vec<Node>>> + Send;
  fn get_configmap(&self, namespace: &str, name: &str) -> impl std::future::Future<Output = Result<Option<ConfigMap>>> + Send;
  fn get_eniconfigs(&self) -> impl std::future::Future<Output = Result<Vec<ENIConfig>>> + Send;
  fn get_resources(&self) -> impl std::future::Future<Output = Result<Vec<StdResource>>> + Send;
}

/// Real AWS client implementation wrapping the SDK clients
pub struct RealAwsClients {
  eks: aws_sdk_eks::Client,
  ec2: aws_sdk_ec2::Client,
  asg: aws_sdk_autoscaling::Client,
}

impl RealAwsClients {
  pub fn new(config: &aws_config::SdkConfig) -> Self {
    Self {
      eks: aws_sdk_eks::Client::new(config),
      ec2: aws_sdk_ec2::Client::new(config),
      asg: aws_sdk_autoscaling::Client::new(config),
    }
  }
}

impl AwsClients for RealAwsClients {
  async fn get_cluster(&self, name: &str) -> Result<Cluster> {
    eks_resources::get_cluster(&self.eks, name).await
  }

  async fn get_subnet_ips(&self, subnet_ids: Vec<String>) -> Result<Vec<VpcSubnet>> {
    eks_resources::get_subnet_ips(&self.ec2, subnet_ids).await
  }

  async fn get_addons(&self, cluster_name: &str) -> Result<Vec<Addon>> {
    eks_resources::get_addons(&self.eks, cluster_name).await
  }

  async fn get_addon_versions(&self, name: &str, kubernetes_version: &str) -> Result<AddonVersion> {
    eks_resources::get_addon_versions(&self.eks, name, kubernetes_version).await
  }

  async fn get_eks_managed_nodegroups(&self, cluster_name: &str) -> Result<Vec<Nodegroup>> {
    eks_resources::get_eks_managed_nodegroups(&self.eks, cluster_name).await
  }

  async fn get_self_managed_nodegroups(&self, cluster_name: &str) -> Result<Vec<AutoScalingGroup>> {
    eks_resources::get_self_managed_nodegroups(&self.asg, cluster_name).await
  }

  async fn get_fargate_profiles(&self, cluster_name: &str) -> Result<Vec<FargateProfile>> {
    eks_resources::get_fargate_profiles(&self.eks, cluster_name).await
  }

  async fn get_launch_template(&self, id: &str) -> Result<LaunchTemplate> {
    eks_resources::get_launch_template(&self.ec2, id).await
  }
}

/// Real Kubernetes client implementation wrapping kube-rs
pub struct RealK8sClients {
  client: kube::Client,
}

impl RealK8sClients {
  pub async fn new(cluster_name: &str) -> Result<Self> {
    match kube::Client::try_default().await {
      Ok(client) => Ok(Self { client }),
      Err(_) => {
        anyhow::bail!(
          "Unable to connect to cluster. Ensure kubeconfig file is present and updated to connect to the cluster.\n\
          Try: aws eks update-kubeconfig --name {cluster_name}"
        );
      }
    }
  }
}

impl K8sClients for RealK8sClients {
  async fn get_nodes(&self) -> Result<Vec<Node>> {
    k8s_resources::get_nodes(&self.client).await
  }

  async fn get_configmap(&self, namespace: &str, name: &str) -> Result<Option<ConfigMap>> {
    k8s_resources::get_configmap(&self.client, namespace, name).await
  }

  async fn get_eniconfigs(&self) -> Result<Vec<ENIConfig>> {
    k8s_resources::get_eniconfigs(&self.client).await
  }

  async fn get_resources(&self) -> Result<Vec<StdResource>> {
    k8s_resources::get_resources(&self.client).await
  }
}
```

**Step 3: Register the module in `lib.rs`**

Add after `mod analysis;`:
```rust
pub mod clients;
```

**Step 4: Verify it compiles**

Run: `cargo check -p eksup`
Expected: compiles with no errors

**Step 5: Commit**

```bash
git add eksup/src/clients.rs eksup/src/lib.rs eksup/src/eks/mod.rs eksup/src/k8s/mod.rs eksup/src/eks/resources.rs
git commit -m "feat: define AwsClients and K8sClients traits with real implementations"
```

---

### Task 5: Refactor findings, analysis, and lib to use traits

Thread the traits through the orchestration layer. Also add `tokio::try_join!` for concurrent findings.

**Files:**
- Modify: `eksup/src/eks/findings.rs`
- Modify: `eksup/src/k8s/findings.rs`
- Modify: `eksup/src/k8s/mod.rs`
- Modify: `eksup/src/analysis.rs`
- Modify: `eksup/src/lib.rs`

**Step 1: Refactor `eks/findings.rs` to use `AwsClients` + `K8sClients`**

Replace all concrete client parameters with trait bounds. Full replacement of the file:

```rust
use anyhow::{Context, Result};
use aws_sdk_eks::types::Cluster;
use serde::{Deserialize, Serialize};

use crate::{
  clients::{AwsClients, K8sClients},
  eks::{checks, resources},
  version,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterFindings {
  pub cluster_health: Vec<checks::ClusterHealthIssue>,
}

pub fn get_cluster_findings(cluster: &Cluster) -> Result<ClusterFindings> {
  let cluster_health = checks::cluster_health(cluster)?;
  Ok(ClusterFindings { cluster_health })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubnetFindings {
  pub control_plane_ips: Vec<checks::InsufficientSubnetIps>,
  pub pod_ips: Vec<checks::InsufficientSubnetIps>,
}

pub async fn get_subnet_findings(
  aws: &(impl AwsClients),
  k8s: &(impl K8sClients),
  cluster: &Cluster,
) -> Result<SubnetFindings> {
  let control_plane_subnet_ids = match cluster.resources_vpc_config() {
    Some(vpc_config) => vpc_config.subnet_ids().to_owned(),
    None => vec![],
  };
  let control_plane_subnet_ips = if control_plane_subnet_ids.is_empty() {
    vec![]
  } else {
    aws.get_subnet_ips(control_plane_subnet_ids).await?
  };

  let eniconfigs = k8s.get_eniconfigs().await?;
  let pod_subnet_ids: Vec<String> = eniconfigs
    .iter()
    .filter_map(|eniconfig| eniconfig.spec.subnet.clone())
    .collect();
  let pod_subnet_ips = if pod_subnet_ids.is_empty() {
    vec![]
  } else {
    aws.get_subnet_ips(pod_subnet_ids).await?
  };

  let control_plane_ips = checks::control_plane_ips(&control_plane_subnet_ips);
  let pod_ips = checks::pod_ips(&pod_subnet_ips, 16, 256);

  Ok(SubnetFindings {
    control_plane_ips,
    pod_ips,
  })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddonFindings {
  pub version_compatibility: Vec<checks::AddonVersionCompatibility>,
  pub health: Vec<checks::AddonHealthIssue>,
}

pub async fn get_addon_findings(
  aws: &(impl AwsClients),
  cluster_name: &str,
  cluster_version: &str,
  target_minor: i32,
) -> Result<AddonFindings> {
  let addons = aws.get_addons(cluster_name).await?;
  let target_k8s_version = version::format_version(target_minor);

  let mut current_versions = std::collections::HashMap::new();
  let mut target_versions = std::collections::HashMap::new();
  for addon in &addons {
    let name = addon.addon_name().unwrap_or_default().to_owned();
    let current = aws.get_addon_versions(&name, cluster_version).await?;
    let target = aws.get_addon_versions(&name, &target_k8s_version).await?;
    current_versions.insert(name.clone(), current);
    target_versions.insert(name, target);
  }

  let version_compatibility = checks::addon_version_compatibility(&addons, &current_versions, &target_versions);
  let health = checks::addon_health(&addons)?;

  Ok(AddonFindings {
    version_compatibility,
    health,
  })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataPlaneFindings {
  pub eks_managed_nodegroup_health: Vec<checks::NodegroupHealthIssue>,
  pub eks_managed_nodegroup_update: Vec<checks::ManagedNodeGroupUpdate>,
  pub self_managed_nodegroup_update: Vec<checks::AutoscalingGroupUpdate>,
  pub al2_ami_deprecation: Vec<checks::Al2AmiDeprecation>,
  pub eks_managed_nodegroups: Vec<String>,
  pub self_managed_nodegroups: Vec<String>,
  pub fargate_profiles: Vec<String>,
}

pub async fn get_data_plane_findings(
  aws: &(impl AwsClients),
  cluster: &Cluster,
  target_minor: i32,
) -> Result<DataPlaneFindings> {
  let cluster_name = cluster.name().unwrap_or_default();

  let eks_mngs = aws.get_eks_managed_nodegroups(cluster_name).await?;
  let self_mngs = aws.get_self_managed_nodegroups(cluster_name).await?;
  let fargate_profiles = aws.get_fargate_profiles(cluster_name).await?;

  let eks_managed_nodegroup_health = checks::eks_managed_nodegroup_health(&eks_mngs)?;
  let al2_ami_deprecation = checks::al2_ami_deprecation(&eks_mngs, target_minor)?;

  let mut eks_managed_nodegroup_update = Vec::new();
  for eks_mng in &eks_mngs {
    let lt = match eks_mng.launch_template() {
      Some(lt_spec) => {
        let lt_id = lt_spec.id().context("Launch template spec missing ID")?;
        Some(aws.get_launch_template(lt_id).await?)
      }
      None => None,
    };
    eks_managed_nodegroup_update.extend(checks::eks_managed_nodegroup_update(eks_mng, lt.as_ref()));
  }

  let mut self_managed_nodegroup_update = Vec::new();
  for self_mng in &self_mngs {
    let lt_spec = self_mng
      .launch_template()
      .context("Launch template not found, launch configuration is not supported")?;
    let lt = aws.get_launch_template(lt_spec.launch_template_id().unwrap_or_default()).await?;
    if let Some(update) = checks::self_managed_nodegroup_update(self_mng, &lt) {
      self_managed_nodegroup_update.push(update);
    }
  }

  Ok(DataPlaneFindings {
    eks_managed_nodegroup_health,
    eks_managed_nodegroup_update,
    self_managed_nodegroup_update,
    al2_ami_deprecation,
    eks_managed_nodegroups: eks_mngs
      .iter()
      .map(|mng| mng.nodegroup_name().unwrap_or_default().to_owned())
      .collect(),
    self_managed_nodegroups: self_mngs
      .iter()
      .map(|asg| asg.auto_scaling_group_name().unwrap_or_default().to_owned())
      .collect(),
    fargate_profiles: fargate_profiles
      .iter()
      .map(|fp| fp.fargate_profile_name().unwrap_or_default().to_owned())
      .collect(),
  })
}
```

**Step 2: Refactor `k8s/findings.rs` to use `K8sClients`**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
  clients::K8sClients,
  k8s::checks::{self, K8sFindings},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct KubernetesFindings {
  pub version_skew: Vec<checks::VersionSkew>,
  pub min_replicas: Vec<checks::MinReplicas>,
  pub min_ready_seconds: Vec<checks::MinReadySeconds>,
  pub readiness_probe: Vec<checks::Probe>,
  pub pod_topology_distribution: Vec<checks::PodTopologyDistribution>,
  pub termination_grace_period: Vec<checks::TerminationGracePeriod>,
  pub docker_socket: Vec<checks::DockerSocket>,
  pub kube_proxy_version_skew: Vec<checks::KubeProxyVersionSkew>,
  pub kube_proxy_ipvs_mode: Vec<checks::KubeProxyIpvsMode>,
  pub ingress_nginx_retirement: Vec<checks::IngressNginxRetirement>,
}

pub async fn get_kubernetes_findings(
  k8s: &(impl K8sClients),
  control_plane_minor: i32,
  target_minor: i32,
) -> Result<KubernetesFindings> {
  let resources = k8s.get_resources().await?;
  let nodes = k8s.get_nodes().await?;
  let kube_proxy_config = k8s.get_configmap("kube-system", "kube-proxy-config").await?;

  let version_skew = checks::version_skew(&nodes, control_plane_minor);
  let min_replicas: Vec<checks::MinReplicas> = resources.iter().filter_map(|s| s.min_replicas()).collect();
  let min_ready_seconds: Vec<checks::MinReadySeconds> =
    resources.iter().filter_map(|s| s.min_ready_seconds()).collect();
  let pod_topology_distribution: Vec<checks::PodTopologyDistribution> =
    resources.iter().filter_map(|s| s.pod_topology_distribution()).collect();
  let readiness_probe: Vec<checks::Probe> = resources.iter().filter_map(|s| s.readiness_probe()).collect();
  let termination_grace_period: Vec<checks::TerminationGracePeriod> =
    resources.iter().filter_map(|s| s.termination_grace_period()).collect();
  let docker_socket: Vec<checks::DockerSocket> = resources
    .iter()
    .filter_map(|s| s.docker_socket().ok().flatten())
    .collect();
  let kube_proxy_version_skew = checks::kube_proxy_version_skew(&resources, control_plane_minor)?;
  let kube_proxy_ipvs_mode = checks::kube_proxy_ipvs_mode(kube_proxy_config.as_ref(), target_minor)?;
  let ingress_nginx_retirement = checks::ingress_nginx_retirement(&resources, target_minor)?;

  Ok(KubernetesFindings {
    version_skew,
    min_replicas,
    min_ready_seconds,
    readiness_probe,
    pod_topology_distribution,
    termination_grace_period,
    docker_socket,
    kube_proxy_version_skew,
    kube_proxy_ipvs_mode,
    ingress_nginx_retirement,
  })
}
```

**Step 3: Remove `get_eniconfigs` re-export from `k8s/mod.rs`**

It's no longer called externally (findings.rs now uses the trait). Change `k8s/mod.rs` to:
```rust
pub(crate) mod checks;
pub(crate) mod findings;
pub(crate) mod resources;

pub use findings::{KubernetesFindings, get_kubernetes_findings};
```

**Step 4: Refactor `analysis.rs` to use traits + `tokio::try_join!`**

```rust
use anyhow::{Context, Result};
use aws_sdk_eks::types::Cluster;
use serde::{Deserialize, Serialize};

use crate::{clients::{AwsClients, K8sClients}, eks, finding::Findings, k8s, version};

#[derive(Debug, Serialize, Deserialize)]
pub struct Results {
  pub cluster: eks::ClusterFindings,
  pub subnets: eks::SubnetFindings,
  pub data_plane: eks::DataPlaneFindings,
  pub addons: eks::AddonFindings,
  pub kubernetes: k8s::KubernetesFindings,
}

impl Results {
  pub fn filter_recommended(&mut self) {
    self.cluster.cluster_health.retain(|f| !f.finding.remediation.is_recommended());
    self.subnets.control_plane_ips.retain(|f| !f.finding.remediation.is_recommended());
    self.subnets.pod_ips.retain(|f| !f.finding.remediation.is_recommended());
    self.addons.health.retain(|f| !f.finding.remediation.is_recommended());
    self.addons.version_compatibility.retain(|f| !f.finding.remediation.is_recommended());
    self.data_plane.eks_managed_nodegroup_health.retain(|f| !f.finding.remediation.is_recommended());
    self.data_plane.eks_managed_nodegroup_update.retain(|f| !f.finding.remediation.is_recommended());
    self.data_plane.self_managed_nodegroup_update.retain(|f| !f.finding.remediation.is_recommended());
    self.data_plane.al2_ami_deprecation.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.version_skew.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.min_replicas.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.min_ready_seconds.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.pod_topology_distribution.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.readiness_probe.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.termination_grace_period.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.docker_socket.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.kube_proxy_version_skew.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.kube_proxy_ipvs_mode.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.ingress_nginx_retirement.retain(|f| !f.finding.remediation.is_recommended());
  }

  pub fn to_stdout_table(&self) -> Result<String> {
    let mut output = String::new();

    output.push_str(&self.subnets.pod_ips.to_stdout_table()?);
    output.push_str(&self.subnets.control_plane_ips.to_stdout_table()?);
    output.push_str(&self.cluster.cluster_health.to_stdout_table()?);
    output.push_str(&self.data_plane.eks_managed_nodegroup_health.to_stdout_table()?);
    output.push_str(&self.addons.health.to_stdout_table()?);
    output.push_str(&self.addons.version_compatibility.to_stdout_table()?);
    output.push_str(&self.data_plane.eks_managed_nodegroup_update.to_stdout_table()?);
    output.push_str(&self.data_plane.self_managed_nodegroup_update.to_stdout_table()?);
    output.push_str(&self.data_plane.al2_ami_deprecation.to_stdout_table()?);
    output.push_str(&self.kubernetes.version_skew.to_stdout_table()?);
    output.push_str(&self.kubernetes.min_replicas.to_stdout_table()?);
    output.push_str(&self.kubernetes.min_ready_seconds.to_stdout_table()?);
    output.push_str(&self.kubernetes.pod_topology_distribution.to_stdout_table()?);
    output.push_str(&self.kubernetes.readiness_probe.to_stdout_table()?);
    output.push_str(&self.kubernetes.termination_grace_period.to_stdout_table()?);
    output.push_str(&self.kubernetes.docker_socket.to_stdout_table()?);
    output.push_str(&self.kubernetes.kube_proxy_version_skew.to_stdout_table()?);
    output.push_str(&self.kubernetes.kube_proxy_ipvs_mode.to_stdout_table()?);
    output.push_str(&self.kubernetes.ingress_nginx_retirement.to_stdout_table()?);

    Ok(output)
  }
}

pub async fn analyze(
  aws: &(impl AwsClients),
  k8s: &(impl K8sClients),
  cluster: &Cluster,
) -> Result<Results> {
  let cluster_name = cluster.name().context("Cluster name missing from API response")?;
  let cluster_version = cluster.version().context("Cluster version missing from API response")?;
  let target_minor = version::get_target_version(cluster_version)?;
  let control_plane_minor = target_minor - 1;

  let cluster_findings = eks::get_cluster_findings(cluster)?;

  let (subnet_findings, addon_findings, dataplane_findings, kubernetes_findings) = tokio::try_join!(
    eks::get_subnet_findings(aws, k8s, cluster),
    eks::get_addon_findings(aws, cluster_name, cluster_version, target_minor),
    eks::get_data_plane_findings(aws, cluster, target_minor),
    k8s::get_kubernetes_findings(k8s, control_plane_minor, target_minor),
  )?;

  Ok(Results {
    cluster: cluster_findings,
    subnets: subnet_findings,
    addons: addon_findings,
    data_plane: dataplane_findings,
    kubernetes: kubernetes_findings,
  })
}
```

**Step 5: Refactor `lib.rs` to construct real clients**

Replace the `analyze` and `create` functions in `lib.rs`. Key changes:
- Remove `get_config` usage of direct SDK client construction
- Use `RealAwsClients` and `RealK8sClients`

```rust
pub async fn analyze(args: Analysis) -> Result<()> {
  let aws_config = get_config(&args.region, &args.profile).await?;
  let aws = clients::RealAwsClients::new(&aws_config);
  let cluster = aws.get_cluster(&args.cluster).await?;
  let cluster_version = cluster.version().context("Cluster version not found")?;

  if version::check_version_supported(cluster_version)?.is_none() {
    println!("Cluster is already at the latest supported version: {cluster_version}");
    println!("Nothing to upgrade at this time");
    return Ok(());
  }

  let k8s = clients::RealK8sClients::new(&args.cluster).await?;
  let mut results = analysis::analyze(&aws, &k8s, &cluster).await?;
  if args.ignore_recommended {
    results.filter_recommended();
  }
  output::output(&results, &args.format, &args.output)?;

  Ok(())
}

pub async fn create(args: Create) -> Result<()> {
  match args.command {
    CreateCommands::Playbook(playbook) => {
      let aws_config = get_config(&playbook.region, &playbook.profile).await?;
      let region = aws_config.region().context("AWS region not configured")?.to_string();

      let aws = clients::RealAwsClients::new(&aws_config);
      let cluster = aws.get_cluster(&playbook.cluster).await?;
      let cluster_version = cluster.version().context("Cluster version not found")?;

      if version::check_version_supported(cluster_version)?.is_none() {
        println!("Cluster is already at the latest supported version: {cluster_version}");
        println!("Nothing to upgrade at this time");
        return Ok(());
      }

      let k8s = clients::RealK8sClients::new(&playbook.cluster).await?;
      let mut results = analysis::analyze(&aws, &k8s, &cluster).await?;
      if playbook.ignore_recommended {
        results.filter_recommended();
      }
      playbook::create(playbook, region, &cluster, results)?;
    }
  }

  Ok(())
}
```

Remove `use aws_sdk_eks;` import (no longer constructing EKS client directly in lib.rs). Keep `use crate::clients;`.

**Step 6: Update module visibility in `lib.rs`**

Make modules pub so integration tests can access them:
```rust
pub mod analysis;
pub mod clients;
pub mod eks;
pub mod finding;
pub mod k8s;
pub mod output;
mod playbook;
pub mod version;
```

(`playbook` stays private — snapshot tests access it through `Results` + output.)

**Step 7: Update `eks/mod.rs` re-exports**

Remove `pub use resources::get_cluster;` (now accessed through AwsClients trait). Keep the findings re-exports:
```rust
pub(crate) mod checks;
pub(crate) mod findings;
pub(crate) mod resources;

pub use findings::{
  AddonFindings, ClusterFindings, DataPlaneFindings, SubnetFindings, get_addon_findings, get_cluster_findings,
  get_data_plane_findings, get_subnet_findings,
};
```

**Step 8: Verify all tests pass**

Run: `cargo test -p eksup`
Expected: All existing + new unit tests pass

**Step 9: Commit**

```bash
git add eksup/src/
git commit -m "refactor: thread AwsClients and K8sClients traits through orchestration layer

Findings, analysis, and lib now use trait-bounded generics instead of
concrete SDK clients. analysis::analyze uses tokio::try_join! for
concurrent findings collection. Module visibility updated for test access."
```

---

### Task 6: Create mock infrastructure

Build the mock structs and shared test fixtures that all integration and e2e tests will use.

**Files:**
- Create: `eksup/tests/common/mod.rs`
- Create: `eksup/tests/common/mock_aws.rs`
- Create: `eksup/tests/common/mock_k8s.rs`
- Create: `eksup/tests/common/fixtures.rs`

**Step 1: Create `eksup/tests/common/mod.rs`**

```rust
pub mod fixtures;
pub mod mock_aws;
pub mod mock_k8s;
```

**Step 2: Create `eksup/tests/common/mock_aws.rs`**

```rust
use std::collections::HashMap;

use anyhow::{Result, bail};
use aws_sdk_autoscaling::types::AutoScalingGroup;
use aws_sdk_eks::types::{Addon, Cluster, FargateProfile, Nodegroup};

use eksup::clients::AwsClients;
use eksup::eks::resources::{AddonVersion, LaunchTemplate, VpcSubnet};

/// Mock AWS client for testing. All fields default to "healthy" empty data.
/// Override specific fields to simulate different cluster states.
#[derive(Clone)]
pub struct MockAwsClients {
  pub cluster: Cluster,
  pub subnet_ips: Vec<VpcSubnet>,
  pub addons: Vec<Addon>,
  pub addon_versions: HashMap<(String, String), AddonVersion>,
  pub nodegroups: Vec<Nodegroup>,
  pub self_managed_nodegroups: Vec<AutoScalingGroup>,
  pub fargate_profiles: Vec<FargateProfile>,
  pub launch_templates: HashMap<String, LaunchTemplate>,
}

impl Default for MockAwsClients {
  fn default() -> Self {
    Self {
      cluster: Cluster::builder()
        .name("test-cluster")
        .version("1.30")
        .build(),
      subnet_ips: vec![],
      addons: vec![],
      addon_versions: HashMap::new(),
      nodegroups: vec![],
      self_managed_nodegroups: vec![],
      fargate_profiles: vec![],
      launch_templates: HashMap::new(),
    }
  }
}

impl AwsClients for MockAwsClients {
  async fn get_cluster(&self, _name: &str) -> Result<Cluster> {
    Ok(self.cluster.clone())
  }

  async fn get_subnet_ips(&self, _subnet_ids: Vec<String>) -> Result<Vec<VpcSubnet>> {
    Ok(self.subnet_ips.clone())
  }

  async fn get_addons(&self, _cluster_name: &str) -> Result<Vec<Addon>> {
    Ok(self.addons.clone())
  }

  async fn get_addon_versions(&self, name: &str, kubernetes_version: &str) -> Result<AddonVersion> {
    let key = (name.to_string(), kubernetes_version.to_string());
    self.addon_versions.get(&key).cloned()
      .ok_or_else(|| anyhow::anyhow!("No mock addon version for {name} @ {kubernetes_version}"))
  }

  async fn get_eks_managed_nodegroups(&self, _cluster_name: &str) -> Result<Vec<Nodegroup>> {
    Ok(self.nodegroups.clone())
  }

  async fn get_self_managed_nodegroups(&self, _cluster_name: &str) -> Result<Vec<AutoScalingGroup>> {
    Ok(self.self_managed_nodegroups.clone())
  }

  async fn get_fargate_profiles(&self, _cluster_name: &str) -> Result<Vec<FargateProfile>> {
    Ok(self.fargate_profiles.clone())
  }

  async fn get_launch_template(&self, id: &str) -> Result<LaunchTemplate> {
    self.launch_templates.get(id).cloned()
      .ok_or_else(|| anyhow::anyhow!("No mock launch template for id {id}"))
  }
}

/// Mock that returns errors for all methods — used for error path testing
pub struct MockAwsClientsError;

impl AwsClients for MockAwsClientsError {
  async fn get_cluster(&self, _name: &str) -> Result<Cluster> { bail!("mock AWS error") }
  async fn get_subnet_ips(&self, _subnet_ids: Vec<String>) -> Result<Vec<VpcSubnet>> { bail!("mock AWS error") }
  async fn get_addons(&self, _cluster_name: &str) -> Result<Vec<Addon>> { bail!("mock AWS error") }
  async fn get_addon_versions(&self, _name: &str, _kubernetes_version: &str) -> Result<AddonVersion> { bail!("mock AWS error") }
  async fn get_eks_managed_nodegroups(&self, _cluster_name: &str) -> Result<Vec<Nodegroup>> { bail!("mock AWS error") }
  async fn get_self_managed_nodegroups(&self, _cluster_name: &str) -> Result<Vec<AutoScalingGroup>> { bail!("mock AWS error") }
  async fn get_fargate_profiles(&self, _cluster_name: &str) -> Result<Vec<FargateProfile>> { bail!("mock AWS error") }
  async fn get_launch_template(&self, _id: &str) -> Result<LaunchTemplate> { bail!("mock AWS error") }
}
```

**Step 3: Create `eksup/tests/common/mock_k8s.rs`**

```rust
use anyhow::{Result, bail};
use k8s_openapi::api::core::v1::ConfigMap;

use eksup::clients::K8sClients;
use eksup::k8s::resources::{ENIConfig, Node, StdResource};

/// Mock K8s client for testing
#[derive(Clone, Default)]
pub struct MockK8sClients {
  pub nodes: Vec<Node>,
  pub configmap: Option<ConfigMap>,
  pub eniconfigs: Vec<ENIConfig>,
  pub resources: Vec<StdResource>,
}

impl K8sClients for MockK8sClients {
  async fn get_nodes(&self) -> Result<Vec<Node>> {
    Ok(self.nodes.clone())
  }

  async fn get_configmap(&self, _namespace: &str, _name: &str) -> Result<Option<ConfigMap>> {
    Ok(self.configmap.clone())
  }

  async fn get_eniconfigs(&self) -> Result<Vec<ENIConfig>> {
    Ok(self.eniconfigs.clone())
  }

  async fn get_resources(&self) -> Result<Vec<StdResource>> {
    Ok(self.resources.clone())
  }
}

/// Mock that returns errors for all methods
pub struct MockK8sClientsError;

impl K8sClients for MockK8sClientsError {
  async fn get_nodes(&self) -> Result<Vec<Node>> { bail!("mock K8s error") }
  async fn get_configmap(&self, _namespace: &str, _name: &str) -> Result<Option<ConfigMap>> { bail!("mock K8s error") }
  async fn get_eniconfigs(&self) -> Result<Vec<ENIConfig>> { bail!("mock K8s error") }
  async fn get_resources(&self) -> Result<Vec<StdResource>> { bail!("mock K8s error") }
}
```

**Step 4: Create `eksup/tests/common/fixtures.rs`**

Reusable test data builders. Provide a "healthy cluster" baseline and helpers for common scenarios:

```rust
use std::collections::{BTreeMap, HashMap, HashSet};

use aws_sdk_eks::types::{
  Addon, Cluster, ClusterHealth, FargateProfile, Nodegroup, ResourcesVpcConfig,
};
use k8s_openapi::api::core::v1::{Container, PodSpec, PodTemplateSpec};

use eksup::eks::resources::{AddonVersion, VpcSubnet};
use eksup::k8s::resources::{Kind, Node, StdMetadata, StdResource, StdSpec};

use super::mock_aws::MockAwsClients;
use super::mock_k8s::MockK8sClients;

/// Builds a healthy cluster at version 1.30 with sufficient IPs
pub fn healthy_aws() -> MockAwsClients {
  MockAwsClients {
    cluster: Cluster::builder()
      .name("test-cluster")
      .version("1.30")
      .health(ClusterHealth::builder().build())
      .resources_vpc_config(
        ResourcesVpcConfig::builder()
          .subnet_ids("subnet-1")
          .subnet_ids("subnet-2")
          .build(),
      )
      .build(),
    subnet_ips: vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 100, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 100, availability_zone_id: "use1-az2".into() },
    ],
    ..Default::default()
  }
}

/// Builds a minimal K8s mock with no resources
pub fn healthy_k8s() -> MockK8sClients {
  MockK8sClients::default()
}

/// Creates a Node with the given version
pub fn make_node(name: &str, minor: i32) -> Node {
  Node {
    name: name.into(),
    labels: None,
    kubelet_version: format!("v1.{minor}.0"),
    minor_version: minor,
  }
}

/// Creates a simple Deployment StdResource
pub fn make_deployment(name: &str, namespace: &str, replicas: i32) -> StdResource {
  StdResource {
    metadata: StdMetadata {
      name: name.into(),
      namespace: namespace.into(),
      kind: Kind::Deployment,
      labels: BTreeMap::new(),
      annotations: BTreeMap::new(),
    },
    spec: StdSpec {
      min_ready_seconds: None,
      replicas: Some(replicas),
      template: Some(PodTemplateSpec {
        metadata: None,
        spec: Some(PodSpec {
          containers: vec![Container {
            name: "app".into(),
            ..Default::default()
          }],
          ..Default::default()
        }),
      }),
    },
  }
}

/// Creates an AddonVersion for mock responses
pub fn make_addon_version(latest: &str, default: &str, supported: &[&str]) -> AddonVersion {
  AddonVersion {
    latest: latest.into(),
    default: default.into(),
    supported_versions: supported.iter().map(|s| s.to_string()).collect(),
  }
}
```

**Step 5: Verify mocks compile**

Create a minimal test file to verify. Create `eksup/tests/smoke.rs`:

```rust
mod common;

use common::{fixtures, mock_aws::MockAwsClients, mock_k8s::MockK8sClients};

#[test]
fn mocks_compile() {
  let _aws = fixtures::healthy_aws();
  let _k8s = fixtures::healthy_k8s();
}
```

Run: `cargo test -p eksup --test smoke`
Expected: PASS

**Step 6: Commit**

```bash
git add eksup/tests/
git commit -m "test: add mock infrastructure and test fixtures"
```

---

### Task 7: Write integration tests

Test the orchestration functions (`get_*_findings` and `analyze`) with mock clients.

**Files:**
- Create: `eksup/tests/integration.rs`

**Step 1: Create `eksup/tests/integration.rs`**

```rust
mod common;

use common::{fixtures, mock_aws::MockAwsClients, mock_k8s::MockK8sClients};
use eksup::eks::resources::VpcSubnet;

// ============================================================================
// Cluster findings
// ============================================================================

#[tokio::test]
async fn cluster_findings_healthy() {
  let aws = fixtures::healthy_aws();
  let result = eksup::eks::get_cluster_findings(&aws.cluster).unwrap();
  assert!(result.cluster_health.is_empty());
}

#[tokio::test]
async fn cluster_findings_with_health_issues() {
  use aws_sdk_eks::types::{Cluster, ClusterHealth, ClusterIssue, ClusterIssueCode};

  let cluster = Cluster::builder()
    .name("test")
    .version("1.30")
    .health(
      ClusterHealth::builder()
        .issues(
          ClusterIssue::builder()
            .code(ClusterIssueCode::Ec2SubnetNotFound)
            .message("Subnet not found")
            .resource_ids("subnet-123")
            .build(),
        )
        .build(),
    )
    .build();

  let result = eksup::eks::get_cluster_findings(&cluster).unwrap();
  assert_eq!(result.cluster_health.len(), 1);
}

// ============================================================================
// Subnet findings
// ============================================================================

#[tokio::test]
async fn subnet_findings_sufficient_ips() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let result = eksup::eks::get_subnet_findings(&aws, &k8s, &aws.cluster).await.unwrap();
  assert!(result.control_plane_ips.is_empty());
  assert!(result.pod_ips.is_empty());
}

#[tokio::test]
async fn subnet_findings_insufficient_control_plane_ips() {
  let mut aws = fixtures::healthy_aws();
  aws.subnet_ips = vec![
    VpcSubnet { id: "subnet-1".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
    VpcSubnet { id: "subnet-2".into(), available_ips: 2, availability_zone_id: "use1-az2".into() },
  ];
  let k8s = fixtures::healthy_k8s();

  let result = eksup::eks::get_subnet_findings(&aws, &k8s, &aws.cluster).await.unwrap();
  assert!(!result.control_plane_ips.is_empty(), "should report insufficient IPs");
}

// ============================================================================
// Addon findings
// ============================================================================

#[tokio::test]
async fn addon_findings_no_addons() {
  let aws = fixtures::healthy_aws();
  let result = eksup::eks::get_addon_findings(&aws, "test-cluster", "1.30", 31).await.unwrap();
  assert!(result.version_compatibility.is_empty());
  assert!(result.health.is_empty());
}

#[tokio::test]
async fn addon_findings_version_incompatible() {
  use aws_sdk_eks::types::Addon;
  use std::collections::HashMap;

  let mut aws = fixtures::healthy_aws();
  aws.addons = vec![
    Addon::builder().addon_name("vpc-cni").addon_version("v1.12.0").build(),
  ];
  aws.addon_versions = HashMap::from([
    (("vpc-cni".into(), "1.30".into()), fixtures::make_addon_version("v1.15.0", "v1.14.0", &["v1.15.0", "v1.14.0"])),
    (("vpc-cni".into(), "1.31".into()), fixtures::make_addon_version("v1.16.0", "v1.15.0", &["v1.16.0", "v1.15.0"])),
  ]);

  let result = eksup::eks::get_addon_findings(&aws, "test-cluster", "1.30", 31).await.unwrap();
  assert_eq!(result.version_compatibility.len(), 1);
}

// ============================================================================
// Data plane findings
// ============================================================================

#[tokio::test]
async fn data_plane_findings_empty() {
  let aws = fixtures::healthy_aws();
  let result = eksup::eks::get_data_plane_findings(&aws, &aws.cluster, 31).await.unwrap();
  assert!(result.eks_managed_nodegroup_health.is_empty());
  assert!(result.eks_managed_nodegroup_update.is_empty());
  assert!(result.self_managed_nodegroup_update.is_empty());
}

// ============================================================================
// Kubernetes findings
// ============================================================================

#[tokio::test]
async fn kubernetes_findings_empty() {
  let k8s = fixtures::healthy_k8s();
  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31).await.unwrap();
  assert!(result.version_skew.is_empty());
  assert!(result.min_replicas.is_empty());
}

#[tokio::test]
async fn kubernetes_findings_version_skew() {
  let k8s = MockK8sClients {
    nodes: vec![fixtures::make_node("node-1", 28)],
    ..Default::default()
  };

  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31).await.unwrap();
  assert_eq!(result.version_skew.len(), 1);
}

#[tokio::test]
async fn kubernetes_findings_workload_issues() {
  let k8s = MockK8sClients {
    resources: vec![fixtures::make_deployment("web", "default", 1)],
    ..Default::default()
  };

  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31).await.unwrap();
  assert!(!result.min_replicas.is_empty(), "1 replica should trigger finding");
  assert!(!result.readiness_probe.is_empty(), "missing probe should trigger finding");
}

// ============================================================================
// Full analysis pipeline
// ============================================================================

#[tokio::test]
async fn analyze_healthy_cluster() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let results = eksup::analysis::analyze(&aws, &k8s, &aws.cluster).await.unwrap();

  assert!(results.cluster.cluster_health.is_empty());
  assert!(results.subnets.control_plane_ips.is_empty());
  assert!(results.addons.version_compatibility.is_empty());
  assert!(results.kubernetes.version_skew.is_empty());
}

#[tokio::test]
async fn analyze_filter_recommended() {
  let k8s = MockK8sClients {
    resources: vec![fixtures::make_deployment("web", "default", 2)],
    nodes: vec![fixtures::make_node("node-1", 29)],
    ..Default::default()
  };
  let aws = fixtures::healthy_aws();

  let mut results = eksup::analysis::analyze(&aws, &k8s, &aws.cluster).await.unwrap();
  let before_skew = results.kubernetes.version_skew.len();
  results.filter_recommended();
  // Version skew of 1 is Recommended, so it should be filtered out
  assert!(results.kubernetes.version_skew.len() < before_skew || before_skew == 0);
}

// ============================================================================
// Error paths
// ============================================================================

#[tokio::test]
async fn analyze_aws_error_propagates() {
  use common::mock_aws::MockAwsClientsError;
  use common::mock_k8s::MockK8sClientsError;

  let cluster = aws_sdk_eks::types::Cluster::builder()
    .name("test")
    .version("1.30")
    .build();

  let result = eksup::analysis::analyze(&MockAwsClientsError, &MockK8sClientsError, &cluster).await;
  assert!(result.is_err(), "should propagate AWS/K8s errors");
}
```

**Step 2: Run integration tests**

Run: `cargo test -p eksup --test integration`
Expected: All tests pass

**Step 3: Remove smoke test**

Delete `eksup/tests/smoke.rs` (served its purpose).

**Step 4: Commit**

```bash
git add eksup/tests/
git commit -m "test: add integration tests for findings and analysis pipeline"
```

---

### Task 8: Write e2e snapshot tests

Test the actual user-facing output using `insta` snapshots.

**Files:**
- Create: `eksup/tests/e2e.rs`

**Step 1: Create `eksup/tests/e2e.rs`**

```rust
mod common;

use common::{fixtures, mock_aws::MockAwsClients, mock_k8s::MockK8sClients};
use eksup::analysis::Results;
use eksup::eks::resources::VpcSubnet;
use eksup::output::Format;

/// Helper: run analysis and return Results
async fn run_analysis(aws: &MockAwsClients, k8s: &MockK8sClients) -> Results {
  eksup::analysis::analyze(aws, k8s, &aws.cluster).await.unwrap()
}

/// Helper: render Results as text
fn render_text(results: &Results) -> String {
  results.to_stdout_table().unwrap()
}

/// Helper: render Results as JSON
fn render_json(results: &Results) -> String {
  serde_json::to_string_pretty(results).unwrap()
}

// ============================================================================
// Healthy cluster
// ============================================================================

#[tokio::test]
async fn snapshot_healthy_cluster_text() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("healthy_cluster_text", output);
}

#[tokio::test]
async fn snapshot_healthy_cluster_json() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let results = run_analysis(&aws, &k8s).await;
  let output = render_json(&results);
  insta::assert_snapshot!("healthy_cluster_json", output);
}

// ============================================================================
// Insufficient control plane IPs
// ============================================================================

#[tokio::test]
async fn snapshot_insufficient_control_plane_ips_text() {
  let mut aws = fixtures::healthy_aws();
  aws.subnet_ips = vec![
    VpcSubnet { id: "subnet-1".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
    VpcSubnet { id: "subnet-2".into(), available_ips: 2, availability_zone_id: "use1-az2".into() },
  ];
  let k8s = fixtures::healthy_k8s();
  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("insufficient_control_plane_ips_text", output);
}

// ============================================================================
// Workload best practices
// ============================================================================

#[tokio::test]
async fn snapshot_workload_issues_text() {
  let aws = fixtures::healthy_aws();
  let k8s = MockK8sClients {
    resources: vec![
      fixtures::make_deployment("web", "default", 1),
      fixtures::make_deployment("api", "backend", 2),
    ],
    ..Default::default()
  };
  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("workload_issues_text", output);
}

#[tokio::test]
async fn snapshot_workload_issues_json() {
  let aws = fixtures::healthy_aws();
  let k8s = MockK8sClients {
    resources: vec![
      fixtures::make_deployment("web", "default", 1),
      fixtures::make_deployment("api", "backend", 2),
    ],
    ..Default::default()
  };
  let results = run_analysis(&aws, &k8s).await;
  let output = render_json(&results);
  insta::assert_snapshot!("workload_issues_json", output);
}

// ============================================================================
// Version skew
// ============================================================================

#[tokio::test]
async fn snapshot_version_skew_text() {
  let aws = fixtures::healthy_aws();
  let k8s = MockK8sClients {
    nodes: vec![
      fixtures::make_node("node-1", 28),
      fixtures::make_node("node-2", 27),
    ],
    ..Default::default()
  };
  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("version_skew_text", output);
}

// ============================================================================
// Mixed findings
// ============================================================================

#[tokio::test]
async fn snapshot_mixed_findings_text() {
  let mut aws = fixtures::healthy_aws();
  aws.subnet_ips = vec![
    VpcSubnet { id: "subnet-1".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
    VpcSubnet { id: "subnet-2".into(), available_ips: 100, availability_zone_id: "use1-az2".into() },
  ];
  let k8s = MockK8sClients {
    nodes: vec![fixtures::make_node("node-1", 28)],
    resources: vec![fixtures::make_deployment("web", "default", 1)],
    ..Default::default()
  };

  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("mixed_findings_text", output);
}

#[tokio::test]
async fn snapshot_mixed_findings_json() {
  let mut aws = fixtures::healthy_aws();
  aws.subnet_ips = vec![
    VpcSubnet { id: "subnet-1".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
    VpcSubnet { id: "subnet-2".into(), available_ips: 100, availability_zone_id: "use1-az2".into() },
  ];
  let k8s = MockK8sClients {
    nodes: vec![fixtures::make_node("node-1", 28)],
    resources: vec![fixtures::make_deployment("web", "default", 1)],
    ..Default::default()
  };

  let results = run_analysis(&aws, &k8s).await;
  let output = render_json(&results);
  insta::assert_snapshot!("mixed_findings_json", output);
}

// ============================================================================
// Filter recommended
// ============================================================================

#[tokio::test]
async fn snapshot_filter_recommended_text() {
  let aws = fixtures::healthy_aws();
  let k8s = MockK8sClients {
    nodes: vec![fixtures::make_node("node-1", 29)],
    resources: vec![fixtures::make_deployment("web", "default", 2)],
    ..Default::default()
  };

  let mut results = run_analysis(&aws, &k8s).await;
  results.filter_recommended();
  let output = render_text(&results);
  insta::assert_snapshot!("filter_recommended_text", output);
}
```

**Step 2: Run snapshot tests to generate initial snapshots**

Run: `cargo test -p eksup --test e2e`
Expected: Tests will fail because no snapshots exist yet.

Run: `cargo insta review` (or `cargo install cargo-insta && cargo insta review`)
Review and accept each snapshot.

Alternatively, run with `INSTA_UPDATE=new`:
```bash
INSTA_UPDATE=new cargo test -p eksup --test e2e
```
Then review the generated files in `eksup/tests/snapshots/`.

**Step 3: Verify snapshots pass**

Run: `cargo test -p eksup --test e2e`
Expected: All snapshot tests pass

**Step 4: Commit**

```bash
git add eksup/tests/e2e.rs eksup/tests/snapshots/
git commit -m "test: add e2e snapshot tests for text and JSON output"
```

---

### Task 9: Final verification and cleanup

**Step 1: Run all tests**

Run: `cargo test -p eksup`
Expected: All unit tests (99 existing + ~15 new), integration tests (~12), and e2e snapshot tests (~9) pass.

**Step 2: Run clippy**

Run: `cargo clippy -p eksup -- -D warnings`
Expected: No warnings

**Step 3: Verify test count**

Run: `cargo test -p eksup 2>&1 | grep "test result"`
Expected: ~135 tests total, 0 failures

**Step 4: Commit any fixes**

If clippy or test issues were found, fix and commit.

**Step 5: Final commit**

```bash
git add -A
git commit -m "test: complete integration and e2e test suite with mock infrastructure

Adds trait-based mocking (AwsClients, K8sClients), purifies 5 async
check functions, and implements snapshot testing for user-facing output.
Coverage now includes orchestration functions, output formatting, and
error paths that were previously untested."
```

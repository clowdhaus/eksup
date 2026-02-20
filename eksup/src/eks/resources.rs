use std::collections::HashSet;

use anyhow::{Context, Result};
use aws_sdk_autoscaling::{
  Client as AsgClient,
  types::{AutoScalingGroup, Filter as AsgFilter},
};
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_servicequotas::Client as SqClient;
use aws_sdk_eks::{
  Client as EksClient,
  types::{Addon, Cluster, FargateProfile, Nodegroup},
};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

/// Describe the cluster to get its full details
pub async fn get_cluster(client: &EksClient, name: &str) -> Result<Cluster> {
  let response = client
    .describe_cluster()
    .name(name)
    .send()
    .await
    .context(format!("Cluster '{name}' not found"))?;

  response.cluster.context(format!("Cluster '{name}' not found in response"))
}

/// Container for the subnet IDs and their total available IPs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VpcSubnet {
  pub id: String,
  pub available_ips: i32,
  pub availability_zone_id: String,
}

/// Describe the subnets provided by ID
///
/// This will show the number of available IPs for evaluating
/// IP contention/exhaustion across the various subnets in use
/// by the control plane ENIs, the nodes, and the pods (when custom
/// networking is enabled)
pub async fn get_subnet_ips(client: &Ec2Client, subnet_ids: Vec<String>) -> Result<Vec<VpcSubnet>> {
  let subnets = client
    .describe_subnets()
    .set_subnet_ids(Some(subnet_ids))
    .send()
    .await
    .context("Failed to describe subnets")?
    .subnets
    .context("Subnets not found")?;

  Ok(
    subnets
      .iter()
      .map(|subnet| {
        let id = subnet.subnet_id().unwrap_or_default().to_string();
        let available_ips = subnet.available_ip_address_count.unwrap_or_default();
        let availability_zone_id = subnet.availability_zone_id().unwrap_or_default().to_string();

        VpcSubnet {
          id,
          available_ips,
          availability_zone_id,
        }
      })
      .collect(),
  )
}

pub async fn get_addons(client: &EksClient, cluster_name: &str) -> Result<Vec<Addon>> {
  let mut addon_names = Vec::new();
  let mut next_token: Option<String> = None;
  loop {
    let mut req = client.list_addons().cluster_name(cluster_name);
    if let Some(token) = &next_token {
      req = req.next_token(token);
    }
    let resp = req.send().await.context("Failed to list addons")?;
    addon_names.extend(resp.addons.unwrap_or_default());
    next_token = resp.next_token;
    if next_token.is_none() {
      break;
    }
  }

  let mut addons = Vec::new();

  for addon_name in &addon_names {
    let response = client
      .describe_addon()
      .cluster_name(cluster_name)
      .addon_name(addon_name)
      .send()
      .await
      .context(format!("Failed to describe addon '{addon_name}'"))?
      .addon;

    if let Some(addon) = response {
      addons.push(addon);
    }
  }

  Ok(addons)
}

#[derive(Clone, Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct AddonVersion {
  /// Latest supported version of the addon
  pub latest: String,
  /// Default version of the addon used by the service
  pub default: String,
  /// Supported versions for the given Kubernetes version
  /// This maintains the ordering of latest version to oldest
  #[tabled(skip)]
  pub supported_versions: HashSet<String>,
}

/// Get the addon version details for the given addon and Kubernetes version
///
/// Returns associated version details for a given addon that, primarily used
/// for version compatibility checks and/or upgrade recommendations
pub async fn get_addon_versions(
  client: &EksClient,
  name: &str,
  kubernetes_version: &str,
) -> Result<AddonVersion> {
  // Get all of the addon versions supported for the given addon and Kubernetes version
  let describe = client
    .describe_addon_versions()
    .addon_name(name)
    .kubernetes_version(kubernetes_version)
    .send()
    .await
    .context(format!("Failed to describe addon versions for '{name}' on Kubernetes {kubernetes_version}"))?;

  // Since we are providing an addon name, we are only concerned with the first and only item
  let addon = describe.addons().first()
    .context(format!("No addon found for '{name}' on Kubernetes version '{kubernetes_version}'"))?;
  let latest_version = addon.addon_versions().first()
    .context(format!("No addon versions found for addon '{name}'"))?
    .addon_version().unwrap_or_default();

  // The default version as specified by the EKS API for a given addon and Kubernetes version
  let default_version = addon
    .addon_versions()
    .iter()
    .find(|v| v.compatibilities().iter().any(|c| c.default_version))
    .and_then(|v| v.addon_version())
    .unwrap_or_default();

  // Get the list of ALL supported version for this addon and Kubernetes version
  // The results maintain the oder of latest version to oldest
  let supported_versions: HashSet<String> = addon
    .addon_versions()
    .iter()
    .map(|v| v.addon_version().unwrap_or_default().to_owned())
    .collect();

  Ok(AddonVersion {
    latest: latest_version.to_owned(),
    default: default_version.to_owned(),
    supported_versions,
  })
}

pub async fn get_eks_managed_nodegroups(client: &EksClient, cluster_name: &str) -> Result<Vec<Nodegroup>> {
  let mut nodegroup_names = Vec::new();
  let mut next_token: Option<String> = None;
  loop {
    let mut req = client.list_nodegroups().cluster_name(cluster_name);
    if let Some(token) = &next_token {
      req = req.next_token(token);
    }
    let resp = req.send().await.context("Failed to list node groups")?;
    nodegroup_names.extend(resp.nodegroups.unwrap_or_default());
    next_token = resp.next_token;
    if next_token.is_none() {
      break;
    }
  }

  let mut nodegroups = Vec::new();

  for nodegroup_name in nodegroup_names {
    let ctx = format!("Failed to describe node group '{nodegroup_name}'");
    let response = client
      .describe_nodegroup()
      .cluster_name(cluster_name)
      .nodegroup_name(nodegroup_name)
      .send()
      .await
      .context(ctx)?
      .nodegroup;

    if let Some(nodegroup) = response {
      nodegroups.push(nodegroup);
    }
  }

  Ok(nodegroups)
}

pub async fn get_self_managed_nodegroups(client: &AsgClient, cluster_name: &str) -> Result<Vec<AutoScalingGroup>> {
  let keys = vec![
    format!("k8s.io/cluster/{cluster_name}"),
    format!("kubernetes.io/cluster/{cluster_name}"),
  ];

  let filter = AsgFilter::builder()
    .set_name(Some("tag-key".to_string()))
    .set_values(Some(keys))
    .build();

  let response = client.describe_auto_scaling_groups().filters(filter).send().await
    .context(format!("Failed to describe Auto Scaling groups for cluster '{cluster_name}'"))?;
  let groups = response.auto_scaling_groups().to_owned();

  // Filter out EKS managed node groups by the EKS MNG applied tag
  Ok(groups.into_iter().filter(|group| {
    group
      .tags()
      .iter()
      .all(|tag| tag.key().unwrap_or_default() != "eks:nodegroup-name")
  }).collect())
}

pub async fn get_fargate_profiles(client: &EksClient, cluster_name: &str) -> Result<Vec<FargateProfile>> {
  let mut profile_names = Vec::new();
  let mut next_token: Option<String> = None;
  loop {
    let mut req = client.list_fargate_profiles().cluster_name(cluster_name);
    if let Some(token) = &next_token {
      req = req.next_token(token);
    }
    let resp = req.send().await.context("Failed to list Fargate profiles")?;
    profile_names.extend(resp.fargate_profile_names.unwrap_or_default());
    next_token = resp.next_token;
    if next_token.is_none() {
      break;
    }
  }

  let mut profiles = Vec::new();

  for profile_name in &profile_names {
    let response = client
      .describe_fargate_profile()
      .cluster_name(cluster_name)
      .fargate_profile_name(profile_name)
      .send()
      .await
      .context(format!("Failed to describe Fargate profile '{profile_name}'"))?
      .fargate_profile;

    if let Some(profile) = response {
      profiles.push(profile);
    }
  }

  Ok(profiles)
}

#[derive(Clone, Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct LaunchTemplate {
  /// Name of the launch template
  #[tabled(skip)]
  pub name: String,
  /// The ID of the launch template
  #[tabled(rename = "LAUNCH TEMP ID")]
  pub id: String,
  /// The version of the launch template currently used/specified in the autoscaling group
  #[tabled(rename = "CURRENT")]
  pub current_version: String,
  /// The latest version of the launch template
  #[tabled(rename = "LATEST")]
  pub latest_version: String,
}

/// Represents the usage vs limit for a service quota
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceQuotaUsage {
  pub quota_name: String,
  pub quota_code: String,
  pub current_usage: f64,
  pub limit: f64,
  pub unit: String,
  pub usage_pct: f64,
}

/// Well-known Service Quotas quota codes
pub mod quota_codes {
  /// EC2 Running On-Demand Standard (A, C, D, H, I, M, R, T, Z) Instances - vCPU limit
  pub const EC2_ON_DEMAND_STANDARD: &str = "L-1216C47A";
  /// EBS General Purpose SSD (gp2) volume storage - TiB limit
  pub const EBS_GP2_STORAGE: &str = "L-D18FCD1D";
  /// EBS General Purpose SSD (gp3) volume storage - TiB limit
  pub const EBS_GP3_STORAGE: &str = "L-7A658B76";
}

/// Get the current quota value for a service
pub async fn get_service_quota(
  client: &SqClient,
  service_code: &str,
  quota_code: &str,
) -> Result<(String, f64, String)> {
  let resp = client
    .get_service_quota()
    .service_code(service_code)
    .quota_code(quota_code)
    .send()
    .await
    .context(format!("Failed to get service quota {service_code}/{quota_code}"))?;

  let quota = resp.quota.context("No quota found in response")?;
  let name = quota.quota_name.unwrap_or_default();
  let value = quota.value.unwrap_or(0.0);
  let unit = quota.unit.map(|u| u.to_string()).unwrap_or_default();

  Ok((name, value, unit))
}

/// Count running on-demand EC2 instance vCPUs in the region
pub async fn get_ec2_on_demand_vcpu_count(client: &Ec2Client) -> Result<f64> {
  let mut total_vcpus: f64 = 0.0;
  let mut next_token: Option<String> = None;

  loop {
    let mut req = client
      .describe_instances()
      .filters(
        aws_sdk_ec2::types::Filter::builder()
          .name("instance-state-name")
          .values("running")
          .build(),
      );
    if let Some(token) = &next_token {
      req = req.next_token(token);
    }
    let resp = req.send().await.context("Failed to describe EC2 instances")?;

    for reservation in resp.reservations() {
      for instance in reservation.instances() {
        // Skip spot instances
        if instance.instance_lifecycle().is_some_and(|l| l.as_str() == "spot") {
          continue;
        }
        if let Some(cpu_options) = instance.cpu_options() {
          let cores = cpu_options.core_count.unwrap_or(1) as f64;
          let threads = cpu_options.threads_per_core.unwrap_or(1) as f64;
          total_vcpus += cores * threads;
        }
      }
    }

    next_token = resp.next_token;
    if next_token.is_none() {
      break;
    }
  }

  Ok(total_vcpus)
}

/// Get total EBS volume storage in TiB for a given volume type (gp2, gp3)
pub async fn get_ebs_volume_storage(client: &Ec2Client, volume_type: &str) -> Result<f64> {
  let mut total_gib: f64 = 0.0;
  let mut next_token: Option<String> = None;

  loop {
    let mut req = client
      .describe_volumes()
      .filters(
        aws_sdk_ec2::types::Filter::builder()
          .name("volume-type")
          .values(volume_type)
          .build(),
      );
    if let Some(token) = &next_token {
      req = req.next_token(token);
    }
    let resp = req.send().await.context(format!("Failed to describe {volume_type} EBS volumes"))?;

    for volume in resp.volumes() {
      total_gib += volume.size.unwrap_or(0) as f64;
    }

    next_token = resp.next_token;
    if next_token.is_none() {
      break;
    }
  }

  // Convert GiB to TiB
  Ok(total_gib / 1024.0)
}

pub async fn get_launch_template(client: &Ec2Client, id: &str) -> Result<LaunchTemplate> {
  let output = client
    .describe_launch_templates()
    .set_launch_template_ids(Some(vec![id.to_string()]))
    .send()
    .await
    .context(format!("Failed to describe launch template '{id}'"))?;

  let lts = output.launch_templates
    .context(format!("No launch templates found for id '{id}'"))?;

  lts
    .into_iter()
    .next()
    .map(|lt| LaunchTemplate {
      name: lt.launch_template_name.unwrap_or_default(),
      id: lt.launch_template_id.unwrap_or_default(),
      current_version: lt.default_version_number.unwrap_or_default().to_string(),
      latest_version: lt.latest_version_number.unwrap_or_default().to_string(),
    })
    .context(format!("Unable to find launch template with id '{id}'"))
}

/// Simplified representation of an EKS cluster insight
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterInsight {
  pub id: String,
  pub name: String,
  pub category: String,
  pub status: String,
  pub status_reason: String,
  pub kubernetes_version: String,
  pub description: String,
  pub recommendation: String,
}

/// List all cluster insights that are not in a PASSING state
pub async fn list_insights(client: &EksClient, cluster_name: &str) -> Result<Vec<String>> {
  use aws_sdk_eks::types::{InsightStatusValue, InsightsFilter};

  let filter = InsightsFilter::builder()
    .statuses(InsightStatusValue::Error)
    .statuses(InsightStatusValue::Warning)
    .statuses(InsightStatusValue::UnknownValue)
    .build();

  let mut insight_ids = Vec::new();
  let mut next_token: Option<String> = None;
  loop {
    let mut req = client
      .list_insights()
      .cluster_name(cluster_name)
      .filter(filter.clone());
    if let Some(token) = &next_token {
      req = req.next_token(token);
    }
    let resp = req.send().await.context("Failed to list cluster insights")?;

    for summary in resp.insights() {
      if let Some(id) = summary.id() {
        insight_ids.push(id.to_string());
      }
    }

    next_token = resp.next_token;
    if next_token.is_none() {
      break;
    }
  }

  Ok(insight_ids)
}

/// Describe a single cluster insight by ID
pub async fn describe_insight(
  client: &EksClient,
  cluster_name: &str,
  insight_id: &str,
) -> Result<ClusterInsight> {
  let resp = client
    .describe_insight()
    .cluster_name(cluster_name)
    .id(insight_id)
    .send()
    .await
    .context(format!("Failed to describe insight '{insight_id}'"))?;

  let insight = resp.insight.context("No insight found in response")?;

  let (status, status_reason) = match insight.insight_status() {
    Some(s) => (
      s.status().map(|v| v.as_str().to_string()).unwrap_or_default(),
      s.reason().unwrap_or_default().to_string(),
    ),
    None => (String::new(), String::new()),
  };

  Ok(ClusterInsight {
    id: insight.id().unwrap_or_default().to_string(),
    name: insight.name().unwrap_or_default().to_string(),
    category: insight.category().map(|c| c.as_str().to_string()).unwrap_or_default(),
    status,
    status_reason,
    kubernetes_version: insight.kubernetes_version().unwrap_or_default().to_string(),
    description: insight.description().unwrap_or_default().to_string(),
    recommendation: insight.recommendation().unwrap_or_default().to_string(),
  })
}

/// Fetch all non-PASSING cluster insights with full details
pub async fn get_cluster_insights(
  client: &EksClient,
  cluster_name: &str,
) -> Result<Vec<ClusterInsight>> {
  let insight_ids = list_insights(client, cluster_name).await?;

  let mut insights = Vec::new();
  for id in &insight_ids {
    let insight = describe_insight(client, cluster_name, id).await?;
    insights.push(insight);
  }

  Ok(insights)
}

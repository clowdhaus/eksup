use anyhow::{Context, Result};
use aws_sdk_eks::types::Cluster;
use serde::{Deserialize, Serialize};

use crate::{
  clients::{AwsClients, K8sClients},
  eks::{checks, resources::quota_codes},
  finding::Code,
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
  aws: &impl AwsClients,
  k8s: &impl K8sClients,
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
  aws: &impl AwsClients,
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
  pub node_ips: Vec<checks::InsufficientSubnetIps>,
  pub eks_managed_nodegroups: Vec<String>,
  pub self_managed_nodegroups: Vec<String>,
  pub fargate_profiles: Vec<String>,
}

pub async fn get_data_plane_findings(
  aws: &impl AwsClients,
  cluster: &Cluster,
  target_minor: i32,
) -> Result<DataPlaneFindings> {
  let cluster_name = cluster.name().unwrap_or_default();

  let eks_mngs = aws.get_eks_managed_nodegroups(cluster_name).await?;
  let self_mngs = aws.get_self_managed_nodegroups(cluster_name).await?;
  let fargate_profiles = aws.get_fargate_profiles(cluster_name).await?;

  let eks_managed_nodegroup_health = checks::eks_managed_nodegroup_health(&eks_mngs)?;
  let al2_ami_deprecation = checks::al2_ami_deprecation(&eks_mngs, target_minor)?;

  // Collect all data plane subnet IDs from nodegroups and Fargate profiles
  let mut data_plane_subnet_ids: Vec<String> = Vec::new();
  for mng in &eks_mngs {
    data_plane_subnet_ids.extend(mng.subnets().iter().map(|s| s.to_string()));
  }
  for fp in &fargate_profiles {
    data_plane_subnet_ids.extend(fp.subnets().iter().map(|s| s.to_string()));
  }
  data_plane_subnet_ids.sort();
  data_plane_subnet_ids.dedup();

  let data_plane_subnet_ips = if data_plane_subnet_ids.is_empty() {
    vec![]
  } else {
    aws.get_subnet_ips(data_plane_subnet_ids).await?
  };

  let node_ips = checks::data_plane_ips(&data_plane_subnet_ips, 30, 100);

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
    node_ips,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceLimitFindings {
  pub ec2_limits: Vec<checks::ServiceLimitFinding>,
  pub ebs_gp2_limits: Vec<checks::ServiceLimitFinding>,
  pub ebs_gp3_limits: Vec<checks::ServiceLimitFinding>,
}

pub async fn get_service_limit_findings(aws: &impl AwsClients) -> Result<ServiceLimitFindings> {
  // EC2 On-Demand vCPU limit
  let ec2_limits = match tokio::try_join!(
    aws.get_service_quota_usage("ec2", quota_codes::EC2_ON_DEMAND_STANDARD),
    aws.get_ec2_on_demand_vcpu_count(),
  ) {
    Ok(((name, limit, unit), current)) => {
      checks::service_limit(Code::AWS003, &name, current, limit, &unit)
        .into_iter().collect()
    }
    Err(e) => {
      tracing::warn!("Unable to check EC2 service limits: {e}");
      vec![]
    }
  };

  // EBS GP2 storage limit
  let ebs_gp2_limits = match tokio::try_join!(
    aws.get_service_quota_usage("ebs", quota_codes::EBS_GP2_STORAGE),
    aws.get_ebs_volume_storage("gp2"),
  ) {
    Ok(((name, limit, unit), current)) => {
      checks::service_limit(Code::AWS004, &name, current, limit, &unit)
        .into_iter().collect()
    }
    Err(e) => {
      tracing::warn!("Unable to check EBS GP2 service limits: {e}");
      vec![]
    }
  };

  // EBS GP3 storage limit
  let ebs_gp3_limits = match tokio::try_join!(
    aws.get_service_quota_usage("ebs", quota_codes::EBS_GP3_STORAGE),
    aws.get_ebs_volume_storage("gp3"),
  ) {
    Ok(((name, limit, unit), current)) => {
      checks::service_limit(Code::AWS005, &name, current, limit, &unit)
        .into_iter().collect()
    }
    Err(e) => {
      tracing::warn!("Unable to check EBS GP3 service limits: {e}");
      vec![]
    }
  };

  Ok(ServiceLimitFindings {
    ec2_limits,
    ebs_gp2_limits,
    ebs_gp3_limits,
  })
}

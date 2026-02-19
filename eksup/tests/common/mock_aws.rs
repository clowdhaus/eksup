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

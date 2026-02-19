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
      Err(e) => {
        anyhow::bail!(
          "Unable to connect to cluster: {e}\n\n\
          Ensure kubeconfig file is present and updated to connect to the cluster.\n\
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

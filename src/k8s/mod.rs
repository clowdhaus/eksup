use k8s_openapi::api::{
  apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
  batch::v1::{CronJob, Job},
  core::v1::{Namespace, Node},
};
pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use kube::{api::Api, Client, CustomResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Returns all of the nodes in the cluster
pub async fn get_nodes(client: &Client) -> Result<Vec<Node>, anyhow::Error> {
  let api: Api<Node> = Api::all(client.clone());
  let nodes = api.list(&Default::default()).await?;

  Ok(nodes.items)
}

pub async fn _get_namespaces(client: &Client) -> Result<Vec<Namespace>, anyhow::Error> {
  let api: Api<Namespace> = Api::all(client.clone());
  let namespaces = api.list(&Default::default()).await?;

  Ok(namespaces.items)
}

pub async fn _get_cronjobs(client: &Client) -> Result<Vec<CronJob>, anyhow::Error> {
  let api: Api<CronJob> = Api::all(client.clone());
  let cronjobs = api.list(&Default::default()).await?;

  Ok(cronjobs.items)
}

pub async fn _get_daemonset(client: &Client) -> Result<Vec<DaemonSet>, anyhow::Error> {
  let api: Api<DaemonSet> = Api::all(client.clone());
  let daemonsets = api.list(&Default::default()).await?;

  Ok(daemonsets.items)
}

pub async fn _get_deployments(client: &Client) -> Result<Vec<Deployment>, anyhow::Error> {
  let api: Api<Deployment> = Api::all(client.clone());
  let deployments = api.list(&Default::default()).await?;

  Ok(deployments.items)
}

pub async fn _get_jobs(client: &Client) -> Result<Vec<Job>, anyhow::Error> {
  let api: Api<Job> = Api::all(client.clone());
  let jobs = api.list(&Default::default()).await?;

  Ok(jobs.items)
}

pub async fn _get_replicasets(client: &Client) -> Result<Vec<ReplicaSet>, anyhow::Error> {
  let api: Api<ReplicaSet> = Api::all(client.clone());
  let replicasets = api.list(&Default::default()).await?;

  Ok(replicasets.items)
}

pub async fn _get_statefulsets(client: &Client) -> Result<Vec<StatefulSet>, anyhow::Error> {
  let api: Api<StatefulSet> = Api::all(client.clone());
  let statefulsets = api.list(&Default::default()).await?;

  Ok(statefulsets.items)
}

/// Custom resource definition for ENIConfig as specified in the AWS VPC CNI
///
/// This makes it possible to query the custom resources in the cluster
/// for extracting information from the ENIConfigs (if present)
/// https://github.com/aws/amazon-vpc-cni-k8s/blob/master/charts/aws-vpc-cni/crds/customresourcedefinition.yaml
#[derive(Clone, CustomResource, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[kube(
  derive = "Default",
  derive = "PartialEq",
  group = "crd.k8s.amazonaws.com",
  kind = "ENIConfig",
  schema = "derived",
  plural = "eniconfigs",
  singular = "eniconfig",
  version = "v1alpha1"
)]
pub struct EniConfigSpec {
  pub subnet: Option<String>,
  pub security_groups: Option<Vec<String>>,
}

/// Returns all of the ENIConfigs in the cluster, if any are present
///
/// This is used to extract the subnet ID(s) to retrieve the number of
/// available IPs in the subnet(s) when custom networking is enabled
pub async fn get_eniconfigs(client: &Client) -> Result<Vec<ENIConfig>, anyhow::Error> {
  let api = Api::<ENIConfig>::all(client.clone());
  let eniconfigs: Vec<ENIConfig> = api.list(&Default::default()).await?.items;

  Ok(eniconfigs)
}

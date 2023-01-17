use k8s_openapi::api::{
  apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
  batch::v1::{CronJob, Job},
  core::v1::{Namespace, Node},
  policy::v1beta1::PodSecurityPolicy,
};
pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use kube::{
  api::{Api, DynamicObject, ResourceExt},
  discovery::{verbs, Discovery, Scope},
  Client, CustomResource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::*;

pub async fn _discover_all(client: &Client) -> Result<(), anyhow::Error> {
  let discovery = Discovery::new(client.clone()).run().await?;
  for group in discovery.groups() {
    for (ar, caps) in group.recommended_resources() {
      if !caps.supports_operation(verbs::LIST) {
        continue;
      }
      let api: Api<DynamicObject> = if caps.scope == Scope::Cluster {
        Api::all_with(client.clone(), &ar)
      } else {
        Api::default_namespaced_with(client.clone(), &ar)
      };

      info!("{}/{} : {}", group.name(), ar.version, ar.kind);
      println!("{}/{} : {}", group.name(), ar.version, ar.kind);

      let list = api.list(&Default::default()).await?;
      for item in list.items {
        let name = item.name_any();
        let ns = item.metadata.namespace.map(|s| s + "/").unwrap_or_default();
        info!("\t\t{ns}{name}");
        println!("\t\t{ns}{name}");
      }
    }
  }

  Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resources {
  pub nodes: Vec<Node>,
  pub namespaces: Vec<Namespace>,
  pub podsecuritypolicies: Vec<PodSecurityPolicy>,
  pub cronjobs: Vec<CronJob>,
  pub daemonsets: Vec<DaemonSet>,
  pub deployments: Vec<Deployment>,
  pub jobs: Vec<Job>,
  pub replicasets: Vec<ReplicaSet>,
  pub statefulsets: Vec<StatefulSet>,
}

pub async fn get_all_resources(client: &Client) -> Result<Resources, anyhow::Error> {
  let nodes = get_nodes(client).await?;
  let namespaces = get_namespaces(client).await?;
  let podsecuritypolicies = get_podsecuritypolicies(client).await?;
  let cronjobs = get_cronjobs(client).await?;
  let daemonsets = get_daemonset(client).await?;
  let deployments = get_deployments(client).await?;
  let jobs = get_jobs(client).await?;
  let replicasets = get_replicasets(client).await?;
  let statefulsets = get_statefulsets(client).await?;

  let resources = Resources {
    nodes,
    namespaces,
    podsecuritypolicies,
    cronjobs,
    daemonsets,
    deployments,
    jobs,
    replicasets,
    statefulsets,
  };

  Ok(resources)
}

/// Returns all of the nodes in the cluster
async fn get_nodes(client: &Client) -> Result<Vec<Node>, anyhow::Error> {
  let api: Api<Node> = Api::all(client.clone());
  let nodes = api.list(&Default::default()).await?;

  Ok(nodes.items)
}

async fn get_namespaces(client: &Client) -> Result<Vec<Namespace>, anyhow::Error> {
  let api: Api<Namespace> = Api::all(client.clone());
  let namespaces = api.list(&Default::default()).await?;

  Ok(namespaces.items)
}

async fn get_podsecuritypolicies(client: &Client) -> Result<Vec<PodSecurityPolicy>, anyhow::Error> {
  let api: Api<PodSecurityPolicy> = Api::all(client.clone());
  let nodes = api.list(&Default::default()).await?;

  Ok(nodes.items)
}

async fn get_cronjobs(client: &Client) -> Result<Vec<CronJob>, anyhow::Error> {
  let api: Api<CronJob> = Api::all(client.clone());
  let cronjobs = api.list(&Default::default()).await?;

  Ok(cronjobs.items)
}

async fn get_daemonset(client: &Client) -> Result<Vec<DaemonSet>, anyhow::Error> {
  let api: Api<DaemonSet> = Api::all(client.clone());
  let daemonsets = api.list(&Default::default()).await?;

  Ok(daemonsets.items)
}

async fn get_deployments(client: &Client) -> Result<Vec<Deployment>, anyhow::Error> {
  let api: Api<Deployment> = Api::all(client.clone());
  let deployments = api.list(&Default::default()).await?;

  Ok(deployments.items)
}

async fn get_jobs(client: &Client) -> Result<Vec<Job>, anyhow::Error> {
  let api: Api<Job> = Api::all(client.clone());
  let jobs = api.list(&Default::default()).await?;

  Ok(jobs.items)
}

async fn get_replicasets(client: &Client) -> Result<Vec<ReplicaSet>, anyhow::Error> {
  let api: Api<ReplicaSet> = Api::all(client.clone());
  let replicasets = api.list(&Default::default()).await?;

  Ok(replicasets.items)
}

async fn get_statefulsets(client: &Client) -> Result<Vec<StatefulSet>, anyhow::Error> {
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

/// Kubernetes workload resources/controllers
///
/// https://kubernetes.io/docs/concepts/workloads/controllers/
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
enum WorkloadResources {
  CronJob,
  DaemonSet,
  Deployment,
  Job,
  ReplicaSet,
  ReplicationController,
  StatefulSet,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct UpdateStrategy {
  _type = String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Spec {
  min_replicas: Option<i32>,
  min_ready_seconds: Option<i32>,

}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DeploymentDetail {
  spec: Spec
}
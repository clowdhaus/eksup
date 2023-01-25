use std::collections::BTreeMap;

use k8s_openapi::api::{apps, batch, core};
use k8s_openapi::api::{
  apps::v1::{DaemonSetSpec, DeploymentSpec, ReplicaSetSpec, StatefulSetSpec},
  batch::v1::{CronJobSpec, JobSpec},
};
use kube::{api::Api, Client, CustomResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
  finding::{self, Findings},
  version,
};

/// Node details as viewed from the Kubernetes API
///
/// Contains information related to the Kubernetes component versions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct NodeFinding {
  pub(crate) name: String,
  pub(crate) kubelet_version: String,
  pub(crate) kubernetes_version: String,
  pub(crate) control_plane_version: String,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

impl Findings for Vec<NodeFinding> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!("{leading_whitespace}:white_check_mark: - No reported findings regarding version skew between the control plane and nodes"));
    }
    let mut counts: BTreeMap<(String, String, String), isize> = BTreeMap::new();
    for node in self {
      *counts
        .entry((
          node.remediation.symbol().to_owned(),
          node.kubernetes_version.to_owned(),
          node.control_plane_version.to_owned(),
        ))
        .or_insert(0) += 1
    }

    let mut summary = String::new();
    summary.push_str(&format!(
      "{leading_whitespace}|  -  | Nodes | Kubelet Version | Control Plane Version |\n"
    ));
    summary.push_str(&format!(
      "{leading_whitespace}| :---: | :---: | :-------------- | :-------------------- |\n"
    ));

    for (k, v) in counts.iter() {
      summary.push_str(&format!(
        "{leading_whitespace}| {sym} | {v} | `v{kube}` | `v{cp}` |\n",
        sym = k.0,
        kube = k.1,
        cp = k.2
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|   -   | Node Name | Kubelet Version | Control Plane Version |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :-------- | :-------------- | :-------------------- |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | `{}` | `v{}` | `v{}` |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.kubernetes_version,
        finding.control_plane_version,
      ))
    }

    Some(format!("{summary}\n{table}\n"))
  }
}

/// Returns all of the nodes in the cluster
pub(crate) async fn version_skew(client: &Client, cluster_version: &str) -> Result<Vec<NodeFinding>, anyhow::Error> {
  let api: Api<core::v1::Node> = Api::all(client.clone());
  let node_list = api.list(&Default::default()).await?;

  let mut findings = vec![];

  for node in &node_list {
    let status = node.status.as_ref().unwrap();
    let node_info = status.node_info.as_ref().unwrap();
    let kubelet_version = node_info.kubelet_version.to_owned();

    let node_minor_version = version::parse_minor(&kubelet_version).unwrap();
    let control_plane_minor_version = version::parse_minor(cluster_version)?;
    let version_skew = control_plane_minor_version - node_minor_version;
    if version_skew == 0 {
      continue;
    }

    // Prior to upgrade, the node version should not be more than 1 version behind
    // the control plane version. If it is, the node must be upgraded before
    // attempting the cluster upgrade
    let remediation = match version_skew {
      1 => finding::Remediation::Recommended,
      _ => finding::Remediation::Required,
    };

    let node = NodeFinding {
      name: node.metadata.name.as_ref().unwrap().to_owned(),
      kubelet_version: kubelet_version.to_owned(),
      kubernetes_version: version::normalize(&kubelet_version).unwrap(),
      control_plane_version: cluster_version.to_owned(),
      remediation,
      fcode: finding::Code::K8S001,
    };

    findings.push(node)
  }

  Ok(findings)
}

/// Custom resource definition for ENIConfig as specified in the AWS VPC CNI
///
/// This makes it possible to query the custom resources in the cluster
/// for extracting information from the ENIConfigs (if present)
/// <https://github.com/aws/amazon-vpc-cni-k8s/blob/master/charts/aws-vpc-cni/crds/customresourcedefinition.yaml>
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
pub(crate) struct EniConfigSpec {
  pub(crate) subnet: Option<String>,
  pub(crate) security_groups: Option<Vec<String>>,
}

/// Returns all of the ENIConfigs in the cluster, if any are present
///
/// This is used to extract the subnet ID(s) to retrieve the number of
/// available IPs in the subnet(s) when custom networking is enabled
pub(crate) async fn get_eniconfigs(client: &Client) -> Result<Vec<ENIConfig>, anyhow::Error> {
  let api = Api::<ENIConfig>::all(client.clone());
  let eniconfigs: Vec<ENIConfig> = api.list(&Default::default()).await?.items;

  Ok(eniconfigs)
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Metadata {
  pub(crate) name: String,
  pub(crate) namespace: String,
  pub(crate) labels: BTreeMap<String, String>,
  pub(crate) annotations: BTreeMap<String, String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Deployment {
  pub(crate) metadata: Metadata,
  pub(crate) spec: DeploymentSpec,
}

pub(crate) async fn _get_deployments(client: &Client) -> Result<Vec<Deployment>, anyhow::Error> {
  let api: Api<apps::v1::Deployment> = Api::all(client.clone());
  let deployment_list = api.list(&Default::default()).await?;

  let deployments = deployment_list
    .items
    .iter()
    .map(|dplmnt| {
      let objmeta = dplmnt.metadata.clone();
      let spec = dplmnt.spec.clone().unwrap();

      let metadata = Metadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      Deployment { metadata, spec }
    })
    .collect();

  Ok(deployments)
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ReplicaSet {
  pub(crate) metadata: Metadata,
  pub(crate) spec: ReplicaSetSpec,
}

async fn _get_replicasets(client: &Client) -> Result<Vec<ReplicaSet>, anyhow::Error> {
  let api: Api<apps::v1::ReplicaSet> = Api::all(client.clone());
  let replicaset_list = api.list(&Default::default()).await?;

  let replicasets = replicaset_list
    .items
    .iter()
    .map(|repl| {
      let objmeta = repl.metadata.clone();
      let spec = repl.spec.clone().unwrap();

      let metadata = Metadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      ReplicaSet { metadata, spec }
    })
    .collect();

  Ok(replicasets)
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StatefulSet {
  pub(crate) metadata: Metadata,
  pub(crate) spec: StatefulSetSpec,
}

async fn _get_statefulsets(client: &Client) -> Result<Vec<StatefulSet>, anyhow::Error> {
  let api: Api<apps::v1::StatefulSet> = Api::all(client.clone());
  let statefulset_list = api.list(&Default::default()).await?;

  let statefulsets = statefulset_list
    .items
    .iter()
    .map(|sset| {
      let objmeta = sset.metadata.clone();
      let spec = sset.spec.clone().unwrap();

      let metadata = Metadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      StatefulSet { metadata, spec }
    })
    .collect();

  Ok(statefulsets)
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DaemonSet {
  pub(crate) metadata: Metadata,
  pub(crate) spec: DaemonSetSpec,
}

async fn _get_daemonset(client: &Client) -> Result<Vec<DaemonSet>, anyhow::Error> {
  let api: Api<apps::v1::DaemonSet> = Api::all(client.clone());
  let daemonset_list = api.list(&Default::default()).await?;

  let daemonsets = daemonset_list
    .items
    .iter()
    .map(|dset| {
      let objmeta = dset.metadata.clone();
      let spec = dset.spec.clone().unwrap();

      let metadata = Metadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      DaemonSet { metadata, spec }
    })
    .collect();

  Ok(daemonsets)
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Job {
  pub(crate) metadata: Metadata,
  pub(crate) spec: JobSpec,
}

async fn _get_jobs(client: &Client) -> Result<Vec<Job>, anyhow::Error> {
  let api: Api<batch::v1::Job> = Api::all(client.clone());
  let job_list = api.list(&Default::default()).await?;

  let jobs = job_list
    .items
    .iter()
    .map(|job| {
      let objmeta = job.metadata.clone();
      let spec = job.spec.clone().unwrap();

      let metadata = Metadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      Job { metadata, spec }
    })
    .collect();

  Ok(jobs)
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CronJob {
  pub(crate) metadata: Metadata,
  pub(crate) spec: CronJobSpec,
}

async fn _get_cronjobs(client: &Client) -> Result<Vec<CronJob>, anyhow::Error> {
  let api: Api<batch::v1::CronJob> = Api::all(client.clone());
  let cronjob_list = api.list(&Default::default()).await?;

  let cronjobs = cronjob_list
    .items
    .iter()
    .map(|cjob| {
      let objmeta = cjob.metadata.clone();
      let spec = cjob.spec.clone().unwrap();

      let metadata = Metadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      CronJob { metadata, spec }
    })
    .collect();

  Ok(cronjobs)
}

// async fn _get_podsecuritypolicies(
//   client: &Client,
// ) -> Result<Vec<policy::v1beta1::PodSecurityPolicy>, anyhow::Error> {
//   let api: Api<policy::v1beta1::PodSecurityPolicy> = Api::all(client.clone());
//   let nodes = api.list(&Default::default()).await?;

//   Ok(nodes.items)
// }

use std::collections::BTreeMap;

use anyhow::Result;
use k8s_openapi::api::{
  apps, batch,
  core::{self, v1::PodTemplateSpec},
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
      return Some(format!(
        "{leading_whitespace}✅ - No reported findings regarding version skew between the control plane and nodes"
      ));
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
pub(crate) async fn version_skew(client: &Client, cluster_version: &str) -> Result<Vec<NodeFinding>> {
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
pub(crate) async fn get_eniconfigs(client: &Client) -> Result<Vec<ENIConfig>> {
  let api = Api::<ENIConfig>::all(client.clone());
  let eniconfigs: Vec<ENIConfig> = api.list(&Default::default()).await?.items;

  Ok(eniconfigs)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MinReplicas {
  pub(crate) resource: Resource,
  /// Number of replicas
  pub(crate) replicas: i32,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

impl Findings for Vec<MinReplicas> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - All relevant Kubernetes workloads have at least 3 replicas specified"
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|  -  | Name | Namespace | Kind | Minimum Replicas |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :--- | :------ | :--- | :--------------- |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{leading_whitespace}| {} | {} | {} | {} | {} |\n",
        finding.remediation.symbol(),
        finding.resource.name,
        finding.resource.namespace,
        finding.resource.kind,
        finding.replicas,
      ))
    }

    Some(format!("{table}\n"))
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MinReadySecondsFinding {
  pub(crate) resource: Resource,
  /// Min ready seconds
  pub(crate) seconds: i32,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PodDisruptionBudgetFinding {
  pub(crate) resource: Resource,
  /// Has pod associated pod disruption budget
  /// TODO - more relevant information than just present?
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PodTopologyDistributionFinding {
  pub(crate) resource: Resource,
  ///
  pub(crate) anti_affinity: Option<String>,
  ///
  pub(crate) toplogy_spread_constraints: Option<String>,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ProbeFinding {
  pub(crate) resource: Resource,
  ///
  pub(crate) readiness: Option<String>,
  pub(crate) liveness: Option<String>,
  pub(crate) startup: Option<String>,

  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct TerminationGracePeriodFinding {
  pub(crate) resource: Resource,
  /// Min ready seconds
  pub(crate) seconds: i32,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DockerSocketFinding {
  pub(crate) resource: Resource,
  ///
  pub(crate) volumes: Vec<String>,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

pub(crate) trait K8sFindings {
  fn get_resource(&self) -> Resource;
  /// K8S002 - check if resources contain a minimum of 3 replicas
  fn min_replicas(&self) -> Option<MinReplicas>;
  /// K8S003 - check if resources contain minReadySeconds > 0
  fn min_ready_seconds(&self) -> Option<MinReadySecondsFinding>;
  // /// K8S004 - check if resources have associated podDisruptionBudgets
  // fn pod_disruption_budget(&self) -> Result<Option<PodDisruptionBudgetFinding>>;
  // /// K8S005 - check if resources have podAntiAffinity or topologySpreadConstraints
  // fn pod_topology_distribution(&self) -> Result<Option<PodTopologyDistributionFinding>>;
  // /// K8S006 - check if resources have readinessProbe
  // fn readiness_probe(&self) -> Result<Option<ProbeFinding>>;
  // /// K8S008 - check if resources use the Docker socket
  // fn docker_socket(&self) -> Result<Option<DockerSocketFinding>>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Resource {
  /// Name of the resources
  pub(crate) name: String,
  /// Namespace where the resource is provisioned
  pub(crate) namespace: String,
  /// Kind of the resource
  pub(crate) kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StdMetadata {
  pub(crate) name: String,
  pub(crate) namespace: String,
  pub(crate) kind: String,
  pub(crate) labels: BTreeMap<String, String>,
  pub(crate) annotations: BTreeMap<String, String>,
}

/// This is a generalized spec used across all resource types that
/// we are inspecting for finding violations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StdSpec {
  /// Minimum number of seconds for which a newly created pod should be ready without any of its container crashing, for it to be considered available. Defaults to 0 (pod will be considered available as soon as it is ready)
  pub min_ready_seconds: Option<i32>,

  /// Number of desired pods. This is a pointer to distinguish between explicit zero and not specified. Defaults to 1.
  pub replicas: Option<i32>,

  /// Template describes the pods that will be created.
  pub template: Option<PodTemplateSpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StdResource {
  pub(crate) metadata: StdMetadata,
  pub(crate) spec: StdSpec,
}

impl K8sFindings for StdResource {
  fn get_resource(&self) -> Resource {
    Resource {
      name: self.metadata.name.clone(),
      namespace: self.metadata.namespace.clone(),
      kind: self.metadata.kind.to_string(),
    }
  }

  fn min_replicas(&self) -> Option<MinReplicas> {
    let replicas = self.spec.replicas;

    match replicas {
      Some(replicas) => {
        if replicas < 3 {
          Some(MinReplicas {
            resource: self.get_resource(),
            replicas,
            remediation: finding::Remediation::Required,
            fcode: finding::Code::K8S002,
          })
        } else {
          None
        }
      }
      None => None,
    }
  }

  fn min_ready_seconds(&self) -> Option<MinReadySecondsFinding> {
    let seconds = self.spec.min_ready_seconds.unwrap_or(0);

    if seconds < 1 {
      Some(MinReadySecondsFinding {
        resource: self.get_resource(),
        seconds,
        remediation: finding::Remediation::Required,
        fcode: finding::Code::K8S003,
      })
    } else {
      None
    }
  }
}

async fn get_deployments(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<apps::v1::Deployment> = Api::all(client.clone());
  let deployment_list = api.list(&Default::default()).await?;

  let deployments = deployment_list
    .items
    .iter()
    .map(|dplmnt| {
      let objmeta = dplmnt.metadata.clone();
      let spec = dplmnt.spec.clone().unwrap();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        kind: "Deployment".to_string(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };

      let spec = StdSpec {
        min_ready_seconds: spec.min_ready_seconds,
        replicas: spec.replicas,
        template: Some(spec.template),
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(deployments)
}

async fn _get_replicasets(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<apps::v1::ReplicaSet> = Api::all(client.clone());
  let replicaset_list = api.list(&Default::default()).await?;

  let replicasets = replicaset_list
    .items
    .iter()
    .map(|repl| {
      let objmeta = repl.metadata.clone();
      let spec = repl.spec.clone().unwrap();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        kind: "ReplicaSet".to_string(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };

      let spec = StdSpec {
        min_ready_seconds: spec.min_ready_seconds,
        replicas: spec.replicas,
        template: spec.template,
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(replicasets)
}

async fn get_statefulsets(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<apps::v1::StatefulSet> = Api::all(client.clone());
  let statefulset_list = api.list(&Default::default()).await?;

  let statefulsets = statefulset_list
    .items
    .iter()
    .map(|sset| {
      let objmeta = sset.metadata.clone();
      let spec = sset.spec.clone().unwrap();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        kind: "StatefulSet".to_string(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };

      let spec = StdSpec {
        min_ready_seconds: spec.min_ready_seconds,
        replicas: spec.replicas,
        template: Some(spec.template),
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(statefulsets)
}

async fn get_daemonsets(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<apps::v1::DaemonSet> = Api::all(client.clone());
  let daemonset_list = api.list(&Default::default()).await?;

  let daemonsets = daemonset_list
    .items
    .iter()
    .map(|dset| {
      let objmeta = dset.metadata.clone();
      let spec = dset.spec.clone().unwrap();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        kind: "DaemonSet".to_string(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };

      let spec = StdSpec {
        min_ready_seconds: spec.min_ready_seconds,
        replicas: None,
        template: Some(spec.template),
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(daemonsets)
}

async fn get_jobs(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<batch::v1::Job> = Api::all(client.clone());
  let job_list = api.list(&Default::default()).await?;

  let jobs = job_list
    .items
    .iter()
    .map(|job| {
      let objmeta = job.metadata.clone();
      let spec = job.spec.clone().unwrap();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        kind: "Job".to_string(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };

      let spec = StdSpec {
        min_ready_seconds: None,
        replicas: None,
        template: Some(spec.template),
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(jobs)
}

async fn get_cronjobs(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<batch::v1::CronJob> = Api::all(client.clone());
  let cronjob_list = api.list(&Default::default()).await?;

  let cronjobs = cronjob_list
    .items
    .iter()
    .map(|cjob| {
      let objmeta = cjob.metadata.clone();
      let spec = cjob.spec.clone().unwrap();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap(),
        namespace: objmeta.namespace.unwrap(),
        kind: "CronJob".to_string(),
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };

      let spec = StdSpec {
        min_ready_seconds: None,
        replicas: None,
        template: match spec.job_template.spec {
          Some(spec) => Some(spec.template),
          None => None,
        },
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(cronjobs)
}

// // https://github.com/kube-rs/kube/issues/428
// // https://github.com/kubernetes/apimachinery/blob/373a5f752d44989b9829888460844849878e1b6e/pkg/apis/meta/v1/helpers.go#L34
// pub(crate) async fn get_pod_disruption_budgets(client: &Client) -> Result<Vec<PodDisruptionBudget>> {
//   let api: Api<policy::v1beta1::PodDisruptionBudget> = Api::all(client.clone());
//   let pdb_list = api.list(&Default::default()).await?;

//   Ok(pdb_list.items)
// }

// async fn get_podsecuritypolicies(
//   client: &Client,
// ) -> Result<Vec<policy::v1beta1::PodSecurityPolicy>> {
//   let api: Api<policy::v1beta1::PodSecurityPolicy> = Api::all(client.clone());
//   let nodes = api.list(&Default::default()).await?;

//   Ok(nodes.items)
// }

pub(crate) async fn get_resources(client: &Client) -> Result<Vec<StdResource>> {
  let cronjobs = get_cronjobs(client).await?;
  let daemonsets = get_daemonsets(client).await?;
  let deployments = get_deployments(client).await?;
  let jobs = get_jobs(client).await?;
  // let replicasets = get_replicasets(client).await?;
  let statefulsets = get_statefulsets(client).await?;

  let mut resources = Vec::new();
  resources.extend(cronjobs);
  resources.extend(daemonsets);
  resources.extend(deployments);
  resources.extend(jobs);
  // resources.extend(replicasets);
  resources.extend(statefulsets);

  Ok(resources)
}

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use k8s_openapi::api::{
  apps, batch,
  core::{self, v1::PodTemplateSpec},
};
use kube::{Client, CustomResource, api::Api};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use tracing::warn;

use crate::{finding, k8s::checks, version};

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
pub struct EniConfigSpec {
  pub subnet: Option<String>,
  pub security_groups: Option<Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Kind {
  DaemonSet,
  Deployment,
  PodSecurityPolicy,
  ReplicaSet,
  ReplicationController,
  StatefulSet,
  CronJob,
  Job,
}

impl std::fmt::Display for Kind {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match *self {
      Kind::DaemonSet => write!(f, "DaemonSet"),
      Kind::Deployment => write!(f, "Deployment"),
      Kind::PodSecurityPolicy => write!(f, "PodSecurityPolicy"),
      Kind::ReplicaSet => write!(f, "ReplicaSet"),
      Kind::ReplicationController => write!(f, "ReplicationController"),
      Kind::StatefulSet => write!(f, "StatefulSet"),
      Kind::CronJob => write!(f, "CronJob"),
      Kind::Job => write!(f, "Job"),
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
  pub name: String,
  pub labels: Option<BTreeMap<String, String>>,
  pub kubelet_version: String,
  pub minor_version: i32,
}

pub async fn get_nodes(client: &Client) -> Result<Vec<Node>> {
  let api: Api<core::v1::Node> = Api::all(client.to_owned());
  let node_list = api.list(&Default::default()).await.context("Failed to list Nodes")?;

  Ok(
    node_list
      .iter()
      .map(|node| {
        let status = node.status.as_ref().unwrap();
        let node_info = status.node_info.as_ref().unwrap();
        let kubelet_version = node_info.kubelet_version.to_owned();
        let minor_version = version::parse_minor(&kubelet_version).unwrap();

        Node {
          name: node.metadata.name.as_ref().unwrap().to_owned(),
          labels: node.metadata.labels.to_owned(),
          kubelet_version,
          minor_version,
        }
      })
      .collect(),
  )
}

/// Returns all of the ENIConfigs in the cluster, if any are present
///
/// This is used to extract the subnet ID(s) to retrieve the number of
/// available IPs in the subnet(s) when custom networking is enabled
pub async fn get_eniconfigs(client: &Client) -> Result<Vec<ENIConfig>> {
  let api = Api::<ENIConfig>::all(client.to_owned());
  let eniconfigs = match api.list(&Default::default()).await {
    Ok(eniconfigs) => eniconfigs.items,
    Err(_) => {
      warn!("Failed to list ENIConfigs");
      vec![]
    }
  };

  Ok(eniconfigs)
}

async fn get_deployments(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<apps::v1::Deployment> = Api::all(client.to_owned());
  let deployment_list = api
    .list(&Default::default())
    .await
    .context("Failed to list Deployments")?;

  let deployments = deployment_list
    .items
    .iter()
    .map(|dplmnt| {
      let objmeta = dplmnt.metadata.clone();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap_or_default(),
        namespace: objmeta.namespace.unwrap_or_default(),
        kind: Kind::Deployment,
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      let spec = match &dplmnt.spec {
        Some(spec) => StdSpec {
          min_ready_seconds: spec.min_ready_seconds,
          replicas: spec.replicas,
          template: Some(spec.template.clone()),
        },
        None => StdSpec {
          min_ready_seconds: None,
          replicas: None,
          template: None,
        },
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(deployments)
}

async fn get_replicasets(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<apps::v1::ReplicaSet> = Api::all(client.to_owned());
  let replicaset_list = api
    .list(&Default::default())
    .await
    .context("Failed to list ReplicaSets")?;

  let replicasets = replicaset_list
    .items
    .iter()
    .filter_map(|repl| match repl.metadata.owner_references {
      None => {
        let objmeta = repl.metadata.clone();

        let metadata = StdMetadata {
          name: objmeta.name.unwrap_or_default(),
          namespace: objmeta.namespace.unwrap_or_default(),
          kind: Kind::ReplicaSet,
          labels: objmeta.labels.unwrap_or_default(),
          annotations: objmeta.annotations.unwrap_or_default(),
        };
        let spec = match &repl.spec {
          Some(spec) => StdSpec {
            min_ready_seconds: spec.min_ready_seconds,
            replicas: spec.replicas,
            template: spec.template.clone(),
          },
          None => StdSpec {
            min_ready_seconds: None,
            replicas: None,
            template: None,
          },
        };

        Some(StdResource { metadata, spec })
      }
      Some(_) => None,
    })
    .collect();

  Ok(replicasets)
}

async fn get_statefulsets(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<apps::v1::StatefulSet> = Api::all(client.to_owned());
  let statefulset_list = api
    .list(&Default::default())
    .await
    .context("Failed to list StatefulSets")?;

  let statefulsets = statefulset_list
    .items
    .iter()
    .map(|sset| {
      let objmeta = sset.metadata.clone();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap_or_default(),
        namespace: objmeta.namespace.unwrap_or_default(),
        kind: Kind::StatefulSet,
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      let spec = match &sset.spec {
        Some(spec) => StdSpec {
          min_ready_seconds: spec.min_ready_seconds,
          replicas: spec.replicas,
          template: Some(spec.template.clone()),
        },
        None => StdSpec {
          min_ready_seconds: None,
          replicas: None,
          template: None,
        },
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(statefulsets)
}

async fn get_daemonsets(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<apps::v1::DaemonSet> = Api::all(client.to_owned());
  let daemonset_list = api
    .list(&Default::default())
    .await
    .context("Failed to list DaemonSets")?;

  let daemonsets = daemonset_list
    .items
    .iter()
    .map(|dset| {
      let objmeta = dset.metadata.clone();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap_or_default(),
        namespace: objmeta.namespace.unwrap_or_default(),
        kind: Kind::DaemonSet,
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      let spec = match &dset.spec {
        Some(spec) => StdSpec {
          min_ready_seconds: spec.min_ready_seconds,
          replicas: None,
          template: Some(spec.template.clone()),
        },
        None => StdSpec {
          min_ready_seconds: None,
          replicas: None,
          template: None,
        },
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(daemonsets)
}

async fn get_jobs(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<batch::v1::Job> = Api::all(client.to_owned());
  let job_list = api.list(&Default::default()).await.context("Failed to list Jobs")?;

  let jobs = job_list
    .items
    .iter()
    .filter_map(|job| match job.metadata.owner_references {
      None => {
        let objmeta = job.metadata.clone();

        let metadata = StdMetadata {
          name: objmeta.name.unwrap_or_default(),
          namespace: objmeta.namespace.unwrap_or_default(),
          kind: Kind::Job,
          labels: objmeta.labels.unwrap_or_default(),
          annotations: objmeta.annotations.unwrap_or_default(),
        };
        let spec = match &job.spec {
          Some(spec) => StdSpec {
            min_ready_seconds: None,
            replicas: None,
            template: Some(spec.template.clone()),
          },
          None => StdSpec {
            min_ready_seconds: None,
            replicas: None,
            template: None,
          },
        };

        Some(StdResource { metadata, spec })
      }
      Some(_) => None,
    })
    .collect();

  Ok(jobs)
}

async fn get_cronjobs(client: &Client) -> Result<Vec<StdResource>> {
  let api: Api<batch::v1::CronJob> = Api::all(client.to_owned());
  let cronjob_list = api.list(&Default::default()).await.context("Failed to list CronJobs")?;

  let cronjobs = cronjob_list
    .items
    .iter()
    .map(|cjob| {
      let objmeta = cjob.metadata.clone();

      let metadata = StdMetadata {
        name: objmeta.name.unwrap_or_default(),
        namespace: objmeta.namespace.unwrap_or_default(),
        kind: Kind::CronJob,
        labels: objmeta.labels.unwrap_or_default(),
        annotations: objmeta.annotations.unwrap_or_default(),
      };
      let spec = match &cjob.spec {
        Some(spec) => StdSpec {
          min_ready_seconds: None,
          replicas: None,
          template: spec.job_template.spec.as_ref().map(|spec| spec.template.clone()),
        },
        None => StdSpec {
          min_ready_seconds: None,
          replicas: None,
          template: None,
        },
      };

      StdResource { metadata, spec }
    })
    .collect();

  Ok(cronjobs)
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct Resource {
  /// Name of the resources
  pub name: String,
  /// Namespace where the resource is provisioned
  pub namespace: String,
  /// Kind of the resource
  pub kind: Kind,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StdMetadata {
  pub name: String,
  pub namespace: String,
  pub kind: Kind,
  pub labels: BTreeMap<String, String>,
  pub annotations: BTreeMap<String, String>,
}

/// This is a generalized spec used across all resource types that
/// we are inspecting for finding violations
#[derive(Debug, Serialize, Deserialize)]
pub struct StdSpec {
  /// Minimum number of seconds for which a newly created pod should be ready without any of its container crashing,
  /// for it to be considered available. Defaults to 0 (pod will be considered available as soon as it is ready)
  pub min_ready_seconds: Option<i32>,

  /// Number of desired pods. This is a pointer to distinguish between explicit zero and not specified. Defaults to 1.
  pub replicas: Option<i32>,

  /// Template describes the pods that will be created.
  pub template: Option<PodTemplateSpec>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StdResource {
  pub metadata: StdMetadata,
  pub spec: StdSpec,
}

impl checks::K8sFindings for StdResource {
  fn get_resource(&self) -> Resource {
    Resource {
      name: self.metadata.name.to_owned(),
      namespace: self.metadata.namespace.to_owned(),
      kind: self.metadata.kind.to_owned(),
    }
  }

  fn min_replicas(&self) -> Option<checks::MinReplicas> {
    let replicas = self.spec.replicas;

    match replicas {
      Some(replicas) => {
        // CoreDNS defaults to 2 replicas
        if self.metadata.name.contains("coredns") && replicas >= 2 {
          return None;
        }
        if replicas < 3 && replicas > 0 {
          let remediation = finding::Remediation::Required;
          let finding = finding::Finding {
            code: finding::Code::K8S002,
            symbol: remediation.symbol(),
            remediation,
          };
          Some(checks::MinReplicas {
            finding,
            resource: self.get_resource(),
            replicas,
          })
        } else {
          None
        }
      }
      None => None,
    }
  }

  fn min_ready_seconds(&self) -> Option<checks::MinReadySeconds> {
    let resource = self.get_resource();

    if [Kind::CronJob, Kind::DaemonSet, Kind::Job].contains(&resource.kind) {
      return None;
    }

    let remediation = match resource.kind {
      Kind::StatefulSet => finding::Remediation::Required,
      _ => finding::Remediation::Recommended,
    };

    let finding = finding::Finding {
      code: finding::Code::K8S003,
      symbol: remediation.symbol(),
      remediation,
    };

    let seconds = self.spec.min_ready_seconds;

    match seconds {
      Some(seconds) => {
        if seconds < 1 {
          Some(checks::MinReadySeconds {
            finding,
            resource: self.get_resource(),
            seconds,
          })
        } else {
          None
        }
      }
      None => {
        // Default value is 0 if a value is not provided
        Some(checks::MinReadySeconds {
          finding,
          resource: self.get_resource(),
          seconds: 0,
        })
      }
    }
  }

  fn readiness_probe(&self) -> Option<checks::Probe> {
    let pod_template = self.spec.template.to_owned();

    let resource = self.get_resource();
    match resource.kind {
      Kind::DaemonSet | Kind::Job | Kind::CronJob => return None,
      _ => (),
    }

    match pod_template {
      Some(pod_template) => {
        let containers = pod_template.spec.unwrap_or_default().containers;

        for container in containers {
          if container.readiness_probe.is_none() {
            let remediation = finding::Remediation::Required;
            let finding = finding::Finding {
              code: finding::Code::K8S006,
              symbol: remediation.symbol(),
              remediation,
            };

            // As soon as we find one container without a readiness probe, we return the finding
            return Some(checks::Probe {
              finding,
              resource: self.get_resource(),
              readiness_probe: container.readiness_probe.is_some(),
            });
          }
        }
        None
      }
      None => None,
    }
  }

  fn pod_topology_distribution(&self) -> Option<checks::PodTopologyDistribution> {
    let pod_template = self.spec.template.to_owned();

    let resource = self.get_resource();
    match resource.kind {
      Kind::DaemonSet | Kind::Job | Kind::CronJob => return None,
      _ => (),
    }

    match pod_template {
      Some(pod_template) => {
        let pod_spec = pod_template.spec.unwrap_or_default();
        if pod_spec.affinity.is_none() && pod_spec.topology_spread_constraints.is_none() {
          let remediation = finding::Remediation::Required;
          let finding = finding::Finding {
            code: finding::Code::K8S005,
            symbol: remediation.symbol(),
            remediation,
          };

          // As soon as we find one container without a readiness probe, we return the finding
          Some(checks::PodTopologyDistribution {
            finding,
            resource: self.get_resource(),
            anti_affinity: pod_spec.affinity.is_some(),
            topology_spread_constraints: pod_spec.topology_spread_constraints.is_some(),
          })
        } else {
          None
        }
      }
      None => None,
    }
  }

  fn termination_grace_period(&self) -> Option<checks::TerminationGracePeriod> {
    let pod_template = self.spec.template.to_owned();

    let resource = self.get_resource();
    match resource.kind {
      // Only applies to StatefulSets
      Kind::StatefulSet => (),
      _ => return None,
    }

    match pod_template {
      Some(pod_template) => {
        let pod_spec = pod_template.spec.unwrap_or_default();
        let termination_grace_period = pod_spec.termination_grace_period_seconds;

        match termination_grace_period {
          Some(termination_grace_period) => {
            if termination_grace_period <= 0 {
              let remediation = finding::Remediation::Required;
              let finding = finding::Finding {
                code: finding::Code::K8S007,
                symbol: remediation.symbol(),
                remediation,
              };

              Some(checks::TerminationGracePeriod {
                finding,
                resource: self.get_resource(),
                termination_grace_period,
              })
            } else {
              // Defaults to 30 seconds if not provided
              None
            }
          }
          None => None,
        }
      }
      None => None,
    }
  }

  fn docker_socket(&self, target_version: &str) -> Option<checks::DockerSocket> {
    let pod_template = self.spec.template.to_owned();

    let target_version = version::parse_minor(target_version).unwrap();
    let remediation = if target_version >= 24 {
      finding::Remediation::Required
    } else {
      finding::Remediation::Recommended
    };

    match pod_template {
      Some(pod_template) => {
        let containers = pod_template.spec.unwrap_or_default().containers;

        for container in containers {
          let volume_mounts = container.volume_mounts.unwrap_or_default();
          for volume_mount in volume_mounts {
            if volume_mount.mount_path.contains("docker.sock") || volume_mount.mount_path.contains("dockershim.sock") {
              let finding = finding::Finding {
                code: finding::Code::K8S008,
                symbol: remediation.symbol(),
                remediation,
              };

              return Some(checks::DockerSocket {
                finding,
                resource: self.get_resource(),
                docker_socket: true,
              });
            }
          }
        }
        None
      }
      None => None,
    }
  }
}

pub async fn get_resources(client: &Client) -> Result<Vec<StdResource>> {
  let cronjobs = get_cronjobs(client).await?;
  let daemonsets = get_daemonsets(client).await?;
  let deployments = get_deployments(client).await?;
  let jobs = get_jobs(client).await?;
  let replicasets = get_replicasets(client).await?;
  let statefulsets = get_statefulsets(client).await?;

  let mut resources = Vec::new();
  resources.extend(cronjobs);
  resources.extend(daemonsets);
  resources.extend(deployments);
  resources.extend(jobs);
  resources.extend(replicasets);
  resources.extend(statefulsets);

  Ok(resources)
}

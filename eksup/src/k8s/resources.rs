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

use crate::{finding::{Code, Finding, Remediation}, k8s::checks, version};

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
      Kind::ReplicaSet => write!(f, "ReplicaSet"),
      Kind::ReplicationController => write!(f, "ReplicationController"),
      Kind::StatefulSet => write!(f, "StatefulSet"),
      Kind::CronJob => write!(f, "CronJob"),
      Kind::Job => write!(f, "Job"),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
      .filter_map(|node| {
        let status = node.status.as_ref()?;
        let node_info = status.node_info.as_ref()?;
        let kubelet_version = node_info.kubelet_version.to_owned();
        let minor_version = version::parse_minor(&kubelet_version).ok()?;

        Some(Node {
          name: node.metadata.name.clone().unwrap_or_default(),
          labels: node.metadata.labels.to_owned(),
          kubelet_version,
          minor_version,
        })
      })
      .collect(),
  )
}

/// Returns a ConfigMap by name from the specified namespace, if it exists
pub async fn get_configmap(client: &Client, namespace: &str, name: &str) -> Result<Option<core::v1::ConfigMap>> {
  let api: Api<core::v1::ConfigMap> = Api::namespaced(client.to_owned(), namespace);
  match api.get_opt(name).await {
    Ok(cm) => Ok(cm),
    Err(e) => {
      warn!("Failed to get ConfigMap {namespace}/{name}: {e}");
      Ok(None)
    }
  }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StdMetadata {
  pub name: String,
  pub namespace: String,
  pub kind: Kind,
  pub labels: BTreeMap<String, String>,
  pub annotations: BTreeMap<String, String>,
}

/// This is a generalized spec used across all resource types that
/// we are inspecting for finding violations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StdSpec {
  /// Minimum number of seconds for which a newly created pod should be ready without any of its container crashing,
  /// for it to be considered available. Defaults to 0 (pod will be considered available as soon as it is ready)
  pub min_ready_seconds: Option<i32>,

  /// Number of desired pods. This is a pointer to distinguish between explicit zero and not specified. Defaults to 1.
  pub replicas: Option<i32>,

  /// Template describes the pods that will be created.
  pub template: Option<PodTemplateSpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
          Some(checks::MinReplicas {
            finding: Finding::new(Code::K8S002, Remediation::Required),
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
      Kind::StatefulSet => Remediation::Required,
      _ => Remediation::Recommended,
    };

    let finding = Finding::new(Code::K8S003, remediation);

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
    let resource = self.get_resource();
    if matches!(resource.kind, Kind::DaemonSet | Kind::Job | Kind::CronJob) {
      return None;
    }

    let pod_template = self.spec.template.as_ref()?;
    let containers = &pod_template.spec.as_ref()?.containers;

    for container in containers {
      if container.readiness_probe.is_none() {
        return Some(checks::Probe {
          finding: Finding::new(Code::K8S006, Remediation::Required),
          resource: self.get_resource(),
          readiness_probe: false,
        });
      }
    }
    None
  }

  fn pod_topology_distribution(&self) -> Option<checks::PodTopologyDistribution> {
    let resource = self.get_resource();
    if matches!(resource.kind, Kind::DaemonSet | Kind::Job | Kind::CronJob) {
      return None;
    }

    let pod_template = self.spec.template.as_ref()?;
    let pod_spec = pod_template.spec.as_ref()?;
    if pod_spec.affinity.is_none() && pod_spec.topology_spread_constraints.is_none() {
      Some(checks::PodTopologyDistribution {
        finding: Finding::new(Code::K8S005, Remediation::Required),
        resource: self.get_resource(),
        anti_affinity: false,
        topology_spread_constraints: false,
      })
    } else {
      None
    }
  }

  fn termination_grace_period(&self) -> Option<checks::TerminationGracePeriod> {
    if !matches!(self.metadata.kind, Kind::StatefulSet) {
      return None;
    }

    let pod_template = self.spec.template.as_ref()?;
    let pod_spec = pod_template.spec.as_ref()?;
    let termination_grace_period = pod_spec.termination_grace_period_seconds?;

    if termination_grace_period <= 0 {
      Some(checks::TerminationGracePeriod {
        finding: Finding::new(Code::K8S007, Remediation::Required),
        resource: self.get_resource(),
        termination_grace_period,
      })
    } else {
      None
    }
  }

  fn docker_socket(&self) -> anyhow::Result<Option<checks::DockerSocket>> {
    let pod_template = match self.spec.template.as_ref() {
      Some(t) => t,
      None => return Ok(None),
    };

    if let Some(containers) = pod_template.spec.as_ref().map(|s| &s.containers) {
      for container in containers {
        for volume_mount in container.volume_mounts.as_deref().unwrap_or_default() {
          if volume_mount.mount_path.contains("docker.sock") || volume_mount.mount_path.contains("dockershim.sock") {
            return Ok(Some(checks::DockerSocket {
              finding: Finding::new(Code::K8S008, Remediation::Required),
              resource: self.get_resource(),
              docker_socket: true,
            }));
          }
        }
      }
    }
    Ok(None)
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::finding::Remediation;
  use crate::k8s::checks::K8sFindings;
  use k8s_openapi::api::core::v1::{
    Affinity, Container, HTTPGetAction, PodAffinityTerm, PodAntiAffinity, PodSpec, PodTemplateSpec,
    Probe as K8sProbe, TopologySpreadConstraint, VolumeMount,
  };
  use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;

  fn make_resource(
    kind: Kind,
    name: &str,
    replicas: Option<i32>,
    min_ready_seconds: Option<i32>,
    template: Option<PodTemplateSpec>,
  ) -> StdResource {
    StdResource {
      metadata: StdMetadata {
        name: name.to_string(),
        namespace: "default".to_string(),
        kind,
        labels: BTreeMap::new(),
        annotations: BTreeMap::new(),
      },
      spec: StdSpec {
        min_ready_seconds,
        replicas,
        template,
      },
    }
  }

  fn basic_template() -> PodTemplateSpec {
    PodTemplateSpec {
      metadata: None,
      spec: Some(PodSpec {
        containers: vec![Container {
          name: "app".to_string(),
          ..Default::default()
        }],
        ..Default::default()
      }),
    }
  }

  fn template_with_anti_affinity() -> PodTemplateSpec {
    PodTemplateSpec {
      metadata: None,
      spec: Some(PodSpec {
        containers: vec![Container {
          name: "app".to_string(),
          ..Default::default()
        }],
        affinity: Some(Affinity {
          pod_anti_affinity: Some(PodAntiAffinity {
            required_during_scheduling_ignored_during_execution: Some(vec![PodAffinityTerm {
              label_selector: Some(LabelSelector::default()),
              topology_key: "kubernetes.io/hostname".to_string(),
              ..Default::default()
            }]),
            ..Default::default()
          }),
          ..Default::default()
        }),
        ..Default::default()
      }),
    }
  }

  fn template_with_spread() -> PodTemplateSpec {
    PodTemplateSpec {
      metadata: None,
      spec: Some(PodSpec {
        containers: vec![Container {
          name: "app".to_string(),
          ..Default::default()
        }],
        topology_spread_constraints: Some(vec![TopologySpreadConstraint {
          max_skew: 1,
          topology_key: "kubernetes.io/hostname".to_string(),
          when_unsatisfiable: "DoNotSchedule".to_string(),
          ..Default::default()
        }]),
        ..Default::default()
      }),
    }
  }

  fn template_with_grace_period(seconds: i64) -> PodTemplateSpec {
    PodTemplateSpec {
      metadata: None,
      spec: Some(PodSpec {
        containers: vec![Container {
          name: "app".to_string(),
          ..Default::default()
        }],
        termination_grace_period_seconds: Some(seconds),
        ..Default::default()
      }),
    }
  }

  // ── min_replicas ──────────────────────────────────────────────────────

  #[test]
  fn min_replicas_below_3() {
    let r = make_resource(Kind::Deployment, "web", Some(2), None, Some(basic_template()));
    let result = r.min_replicas();
    assert!(result.is_some());
    let finding = result.unwrap();
    assert_eq!(finding.replicas, 2);
    assert!(matches!(finding.finding.remediation, Remediation::Required));
  }

  #[test]
  fn min_replicas_at_3() {
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(basic_template()));
    assert!(r.min_replicas().is_none());
  }

  #[test]
  fn min_replicas_statefulset_1() {
    let r = make_resource(Kind::StatefulSet, "db", Some(1), None, Some(basic_template()));
    let result = r.min_replicas();
    assert!(result.is_some());
    assert_eq!(result.unwrap().replicas, 1);
  }

  #[test]
  fn min_replicas_replicaset_0() {
    let r = make_resource(Kind::ReplicaSet, "old", Some(0), None, Some(basic_template()));
    assert!(r.min_replicas().is_none());
  }

  #[test]
  fn min_replicas_job_skipped() {
    let r = make_resource(Kind::Job, "batch", None, None, Some(basic_template()));
    assert!(r.min_replicas().is_none());
  }

  #[test]
  fn min_replicas_none() {
    let r = make_resource(Kind::Deployment, "web", None, None, Some(basic_template()));
    assert!(r.min_replicas().is_none());
  }

  // ── min_ready_seconds ─────────────────────────────────────────────────

  #[test]
  fn min_ready_seconds_zero() {
    let r = make_resource(Kind::Deployment, "web", Some(3), Some(0), Some(basic_template()));
    let result = r.min_ready_seconds();
    assert!(result.is_some());
    assert_eq!(result.unwrap().seconds, 0);
  }

  #[test]
  fn min_ready_seconds_positive() {
    let r = make_resource(Kind::Deployment, "web", Some(3), Some(5), Some(basic_template()));
    assert!(r.min_ready_seconds().is_none());
  }

  #[test]
  fn min_ready_seconds_none() {
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(basic_template()));
    let result = r.min_ready_seconds();
    assert!(result.is_some());
    assert_eq!(result.unwrap().seconds, 0);
  }

  #[test]
  fn min_ready_seconds_deployment_recommended() {
    let r = make_resource(Kind::Deployment, "web", Some(3), Some(0), Some(basic_template()));
    let result = r.min_ready_seconds().unwrap();
    assert!(matches!(result.finding.remediation, Remediation::Recommended));
  }

  #[test]
  fn min_ready_seconds_statefulset_required() {
    let r = make_resource(Kind::StatefulSet, "db", Some(3), Some(0), Some(basic_template()));
    let result = r.min_ready_seconds().unwrap();
    assert!(matches!(result.finding.remediation, Remediation::Required));
  }

  // ── pod_topology_distribution ─────────────────────────────────────────

  #[test]
  fn topology_neither() {
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(basic_template()));
    let result = r.pod_topology_distribution();
    assert!(result.is_some());
    let ptd = result.unwrap();
    assert!(!ptd.anti_affinity);
    assert!(!ptd.topology_spread_constraints);
  }

  #[test]
  fn topology_anti_affinity() {
    let r = make_resource(
      Kind::Deployment,
      "web",
      Some(3),
      None,
      Some(template_with_anti_affinity()),
    );
    assert!(r.pod_topology_distribution().is_none());
  }

  #[test]
  fn topology_spread() {
    let r = make_resource(
      Kind::Deployment,
      "web",
      Some(3),
      None,
      Some(template_with_spread()),
    );
    assert!(r.pod_topology_distribution().is_none());
  }

  #[test]
  fn topology_both() {
    let mut tmpl = template_with_anti_affinity();
    if let Some(ref mut spec) = tmpl.spec {
      spec.topology_spread_constraints = Some(vec![TopologySpreadConstraint {
        max_skew: 1,
        topology_key: "kubernetes.io/hostname".to_string(),
        when_unsatisfiable: "DoNotSchedule".to_string(),
        ..Default::default()
      }]);
    }
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(tmpl));
    assert!(r.pod_topology_distribution().is_none());
  }

  #[test]
  fn topology_daemonset_skipped() {
    let r = make_resource(Kind::DaemonSet, "agent", None, None, Some(basic_template()));
    assert!(r.pod_topology_distribution().is_none());
  }

  #[test]
  fn topology_job_skipped() {
    let r = make_resource(Kind::Job, "batch", None, None, Some(basic_template()));
    assert!(r.pod_topology_distribution().is_none());
  }

  // ── readiness_probe ───────────────────────────────────────────────────

  #[test]
  fn readiness_probe_missing() {
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(basic_template()));
    let result = r.readiness_probe();
    assert!(result.is_some());
    assert!(!result.unwrap().readiness_probe);
  }

  #[test]
  fn readiness_probe_present() {
    let tmpl = PodTemplateSpec {
      metadata: None,
      spec: Some(PodSpec {
        containers: vec![Container {
          name: "app".to_string(),
          readiness_probe: Some(K8sProbe {
            http_get: Some(HTTPGetAction {
              path: Some("/healthz".to_string()),
              port: k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(8080),
              ..Default::default()
            }),
            ..Default::default()
          }),
          ..Default::default()
        }],
        ..Default::default()
      }),
    };
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(tmpl));
    assert!(r.readiness_probe().is_none());
  }

  #[test]
  fn readiness_probe_partial() {
    let tmpl = PodTemplateSpec {
      metadata: None,
      spec: Some(PodSpec {
        containers: vec![
          Container {
            name: "app".to_string(),
            readiness_probe: Some(K8sProbe {
              http_get: Some(HTTPGetAction {
                path: Some("/healthz".to_string()),
                port: k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(8080),
                ..Default::default()
              }),
              ..Default::default()
            }),
            ..Default::default()
          },
          Container {
            name: "sidecar".to_string(),
            ..Default::default()
          },
        ],
        ..Default::default()
      }),
    };
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(tmpl));
    assert!(r.readiness_probe().is_some());
  }

  #[test]
  fn readiness_probe_daemonset_skipped() {
    let r = make_resource(Kind::DaemonSet, "agent", None, None, Some(basic_template()));
    assert!(r.readiness_probe().is_none());
  }

  #[test]
  fn readiness_probe_job_skipped() {
    let r = make_resource(Kind::Job, "batch", None, None, Some(basic_template()));
    assert!(r.readiness_probe().is_none());
  }

  // ── termination_grace_period ──────────────────────────────────────────

  #[test]
  fn termination_grace_period_zero() {
    let r = make_resource(
      Kind::StatefulSet,
      "db",
      Some(3),
      None,
      Some(template_with_grace_period(0)),
    );
    let result = r.termination_grace_period();
    assert!(result.is_some());
    assert_eq!(result.unwrap().termination_grace_period, 0);
  }

  #[test]
  fn termination_grace_period_positive() {
    let r = make_resource(
      Kind::StatefulSet,
      "db",
      Some(3),
      None,
      Some(template_with_grace_period(30)),
    );
    assert!(r.termination_grace_period().is_none());
  }

  #[test]
  fn termination_grace_period_none() {
    let r = make_resource(Kind::StatefulSet, "db", Some(3), None, Some(basic_template()));
    assert!(r.termination_grace_period().is_none());
  }

  #[test]
  fn termination_grace_period_deployment_skipped() {
    let r = make_resource(
      Kind::Deployment,
      "web",
      Some(3),
      None,
      Some(template_with_grace_period(0)),
    );
    assert!(r.termination_grace_period().is_none());
  }

  // ── docker_socket ─────────────────────────────────────────────────────

  #[test]
  fn docker_socket_not_mounted() {
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(basic_template()));
    assert!(r.docker_socket().unwrap().is_none());
  }

  #[test]
  fn docker_socket_docker_sock() {
    let tmpl = PodTemplateSpec {
      metadata: None,
      spec: Some(PodSpec {
        containers: vec![Container {
          name: "app".to_string(),
          volume_mounts: Some(vec![VolumeMount {
            name: "docker".to_string(),
            mount_path: "/var/run/docker.sock".to_string(),
            ..Default::default()
          }]),
          ..Default::default()
        }],
        ..Default::default()
      }),
    };
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(tmpl));
    let result = r.docker_socket().unwrap();
    assert!(result.is_some());
    assert!(result.unwrap().docker_socket);
  }

  #[test]
  fn docker_socket_dockershim_sock() {
    let tmpl = PodTemplateSpec {
      metadata: None,
      spec: Some(PodSpec {
        containers: vec![Container {
          name: "app".to_string(),
          volume_mounts: Some(vec![VolumeMount {
            name: "dockershim".to_string(),
            mount_path: "/var/run/dockershim.sock".to_string(),
            ..Default::default()
          }]),
          ..Default::default()
        }],
        ..Default::default()
      }),
    };
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(tmpl));
    let result = r.docker_socket().unwrap();
    assert!(result.is_some());
    assert!(result.unwrap().docker_socket);
  }

  #[test]
  fn docker_socket_other_mount() {
    let tmpl = PodTemplateSpec {
      metadata: None,
      spec: Some(PodSpec {
        containers: vec![Container {
          name: "app".to_string(),
          volume_mounts: Some(vec![VolumeMount {
            name: "logs".to_string(),
            mount_path: "/var/log".to_string(),
            ..Default::default()
          }]),
          ..Default::default()
        }],
        ..Default::default()
      }),
    };
    let r = make_resource(Kind::Deployment, "web", Some(3), None, Some(tmpl));
    assert!(r.docker_socket().unwrap().is_none());
  }

  #[test]
  fn docker_socket_no_template() {
    let r = make_resource(Kind::Deployment, "web", Some(3), None, None);
    assert!(r.docker_socket().unwrap().is_none());
  }

  // ── node ──────────────────────────────────────────────────────────────

  #[test]
  fn node_struct_construction() {
    let node = Node {
      name: "ip-10-0-1-42".to_string(),
      labels: Some(BTreeMap::from([(
        "node.kubernetes.io/instance-type".to_string(),
        "m5.xlarge".to_string(),
      )])),
      kubelet_version: "v1.28.3".to_string(),
      minor_version: 28,
    };
    assert_eq!(node.name, "ip-10-0-1-42");
    assert!(node
      .labels
      .as_ref()
      .unwrap()
      .contains_key("node.kubernetes.io/instance-type"));
    assert_eq!(node.kubelet_version, "v1.28.3");
  }

  #[test]
  fn node_minor_version() {
    let node = Node {
      name: "worker-1".to_string(),
      labels: None,
      kubelet_version: "v1.30.0".to_string(),
      minor_version: 30,
    };
    assert_eq!(node.minor_version, 30);
  }
}

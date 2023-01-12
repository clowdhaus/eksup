use k8s_openapi::api::core::v1::Node;
use kube::{api::Api, Client, CustomResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Returns all of the nodes in the cluster
pub async fn get_nodes(client: &Client) -> Result<Vec<Node>, anyhow::Error> {
  let api_nodes: Api<Node> = Api::all(client.clone());
  let nodes = api_nodes.list(&Default::default()).await?;

  Ok(nodes.items)
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
pub async fn get_eniconfigs(client: &Client) -> Result<Option<Vec<ENIConfig>>, anyhow::Error> {
  let api = Api::<ENIConfig>::all(client.clone());
  let configs: Vec<ENIConfig> = api.list(&Default::default()).await?.items;

  if configs.is_empty() {
    return Ok(None);
  }

  Ok(Some(configs))
}

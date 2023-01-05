use std::env;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_autoscaling::{
  model::AutoScalingGroup, model::Filter as AsgFilter, Client as AsgClient,
};
use aws_sdk_ec2::{model::Subnet, Client as Ec2Client};
use aws_sdk_eks::{model::Cluster as EksCluster, model::Nodegroup, Client as EksClient};
use aws_types::region::Region;

pub async fn get_shared_config(region: Option<String>) -> aws_config::SdkConfig {
  // TODO - fix this ugliness
  let region_provider = match region {
    Some(region) => RegionProviderChain::first_try(Region::new(region)).or_default_provider(),
    None => RegionProviderChain::first_try(env::var("AWS_REGION").ok().map(Region::new))
      .or_default_provider(),
  };

  aws_config::from_env().region(region_provider).load().await
}

pub async fn get_cluster(client: &EksClient, name: &str) -> Result<EksCluster, anyhow::Error> {
  let req = client.describe_cluster().name(name);
  let resp = req.send().await?;

  // TODO - handle error check here for cluster not found
  let cluster = resp
    .cluster
    .unwrap_or_else(|| panic!("Cluster {} not found", name));

  Ok(cluster)
}

pub async fn get_subnets(
  client: &Ec2Client,
  subnet_ids: Vec<String>,
) -> Result<Vec<Subnet>, anyhow::Error> {
  let subnets = client
    .describe_subnets()
    .set_subnet_ids(Some(subnet_ids))
    .send()
    .await?
    .subnets
    .unwrap();

  Ok(subnets)
}

pub async fn get_eks_managed_node_groups(
  client: &EksClient,
  cluster_name: &str,
) -> Result<Option<Vec<Nodegroup>>, anyhow::Error> {
  let nodegroup_names = client
    .list_nodegroups()
    .cluster_name(cluster_name)
    .max_results(100)
    .send()
    .await?
    .nodegroups
    .unwrap_or_default();

  let mut nodegroups = Vec::new();

  for nodegroup_name in nodegroup_names {
    let response = client
      .describe_nodegroup()
      .cluster_name(cluster_name)
      .nodegroup_name(nodegroup_name)
      .send()
      .await?
      .nodegroup;

    if let Some(nodegroup) = response {
      nodegroups.push(nodegroup);
    }
  }

  Ok(Some(nodegroups))
}

// TODO - querying on tags will return EKS managed node groups as well
// TODO - We will need to de-dupe
pub async fn get_self_managed_node_groups(
  client: &AsgClient,
  cluster_name: &str,
) -> Result<Option<Vec<AutoScalingGroup>>, anyhow::Error> {
  let keys = vec![
    format!("k8s.io/cluster/{}", cluster_name),
    format!("kubernetes.io/cluster/{}", cluster_name),
  ];

  let filter = AsgFilter::builder()
    .set_name(Some("tag-key".to_string()))
    .set_values(Some(keys))
    .build();

  let response = client
    .describe_auto_scaling_groups()
    .filters(filter)
    .send()
    .await?;
  let groups = response.auto_scaling_groups().map(|groups| groups.to_vec());

  Ok(groups)
}

use aws_sdk_ec2::{model::Filter as Ec2Filter, model::Subnet, Client as Ec2Client};
use aws_sdk_eks::{model::Cluster as EksCluster, Client as EksClient};
// use serde::{Deserialize, Serialize};

pub async fn get_cluster(client: &EksClient, name: &str) -> Result<EksCluster, anyhow::Error> {
  let req = client.describe_cluster().name(name);
  let resp = req.send().await?;

  // TODO - handle error check here for cluster not found
  let cluster = resp
    .cluster
    .unwrap_or_else(|| panic!("Cluster {} not found", name));

  Ok(cluster)
}

// #[derive(Serialize, Deserialize, Debug)]
// struct Subnet {
//   id: String,

//   available_ips: u32,

//   total_ips: u32,
// }

pub async fn get_subnets(
  client: &Ec2Client,
  subnet_ids: Vec<String>,
) -> Result<Vec<Subnet>, anyhow::Error> {
  let filter = Ec2Filter::builder()
    .set_name(Some("subnet-ids".to_string()))
    .set_values(Some(subnet_ids))
    .build();

  let subnets = client
    .describe_subnets()
    .filters(filter)
    .send()
    .await?
    .subnets
    .unwrap();

  Ok(subnets)
}
